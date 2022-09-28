use alloc::collections::BinaryHeap;
use core::{
    cell::RefCell,
    cmp::Reverse,
    ops::AddAssign,
    sync::atomic::{AtomicU64, Ordering},
};
use crossbeam_channel::{bounded, select, Receiver, Sender};
use crossbeam_utils::CachePadded;
use std::collections::HashMap;

use crate::{closer::Closer, ref_counter::RefCounter};

enum Index {
    Single(u64),
    Multiple(Vec<u64>),
}

/// mark contains one of more indices, along with a done boolean to indicate the
/// status of the index: begin or done. It also contains waiters, who could be
/// waiting for the watermark to reach >= a certain index.
struct Mark {
    // Either this is an (index, waiter) pair or (index, done) or (indices, done).
    index: Index,
    waiter_tx: Option<Sender<()>>,
    done: bool,
}

struct HotData {
    done_until: CachePadded<AtomicU64>,
    last_index: CachePadded<AtomicU64>,
    mark_rx: Receiver<Mark>,
    name: &'static str,
}

impl HotData {
    #[inline]
    fn new(name: &'static str, mark_rx: Receiver<Mark>) -> Self {
        Self {
            done_until: CachePadded::new(AtomicU64::new(0)),
            last_index: CachePadded::new(AtomicU64::new(0)),
            mark_rx,
            name,
        }
    }

    #[inline]
    fn done_until(&self) -> u64 {
        self.done_until.load(Ordering::SeqCst)
    }
}

/// WaterMark is used to keep track of the minimum un-finished index.  Typically, an index k becomes
/// finished or "done" according to a WaterMark once Done(k) has been called
///   1. as many times as Begin(k) has, AND
///   2. a positive number of times.
///
/// An index may also become "done" by calling SetDoneUntil at a time such that it is not
/// inter-mingled with Begin/Done calls.
///
/// Since doneUntil and lastIndex addresses are passed to sync/atomic packages, we ensure that they
/// are 64-bit aligned by putting them at the beginning of the structure.
pub struct Watermark {
    data: RefCounter<HotData>,
    mark_tx: Sender<Mark>,
}

impl Watermark {
    pub fn new(name: &'static str) -> (Self, WatermarkProcessor) {
        let (mark_tx, mark_rx) = bounded(100);
        let hot_data = RefCounter::new(HotData::new(name, mark_rx));
        (
            Self {
                data: hot_data.clone(),
                mark_tx,
            },
            WatermarkProcessor { data: hot_data },
        )
    }

    #[inline]
    pub fn name(&self) -> &'static str {
        self.data.name
    }

    /// Sets the last index to the given value.
    #[inline]
    pub fn begin(&self, index: u64) {
        self.data.last_index.store(index, Ordering::SeqCst);
        self.mark_tx
            .send(Mark {
                index: Index::Single(index),
                waiter_tx: None,
                done: false,
            })
            .unwrap();
    }

    /// Works like [`Watermark::begin`] but accepts multiple indices.
    #[inline]
    pub fn begin_many(&self, indices: Vec<u64>) {
        self.data
            .last_index
            .store(indices.len() as u64, Ordering::SeqCst);
        self.mark_tx
            .send(Mark {
                index: Index::Multiple(indices),
                waiter_tx: None,
                done: false,
            })
            .unwrap();
    }

    /// Sets a single index as done.
    #[inline]
    pub fn done(&self, index: u64) {
        self.mark_tx
            .send(Mark {
                index: Index::Single(index),
                waiter_tx: None,
                done: true,
            })
            .unwrap();
    }

    /// Works like [`Watermark::done`] but accepts multiple indices.
    #[inline]
    pub fn done_many(&self, index: u64) {
        self.mark_tx
            .send(Mark {
                index: Index::Single(index),
                waiter_tx: None,
                done: true,
            })
            .unwrap();
    }

    /// Returns the maximum index that has the property that all indices
    /// less than or equal to it are done.
    #[inline]
    pub fn done_until(&self) -> u64 {
        self.data.done_until()
    }

    /// Sets the maximum index that has the property that all indices
    /// less than or equal to it are done.
    #[inline]
    pub fn set_done_until(&self, val: u64) {
        self.data.done_until.store(val, Ordering::SeqCst);
    }

    /// Returns the last index for which Begin has been called.
    #[inline]
    pub fn last_index(&self) -> u64 {
        self.data.last_index.load(Ordering::SeqCst)
    }

    /// Waits until the given index is marked as done.
    #[inline]
    pub fn wait_for_mark(&self, index: u64) {
        if self.done_until() >= index {
            return;
        }

        let (tx, rx) = bounded(1);
        self.mark_tx
            .send(Mark {
                index: Index::Single(index),
                waiter_tx: Some(tx),
                done: false,
            })
            .unwrap();

        let _ = rx.recv();
    }
}

/// Processor for [`Watermark`]
pub struct WatermarkProcessor {
    data: RefCounter<HotData>,
}

impl WatermarkProcessor {
    pub fn spawn(self, closer: Closer) {
        std::thread::spawn(move || {
            scopeguard::defer!(closer.done());
            let mut heap: BinaryHeap<Reverse<u64>> = BinaryHeap::new();

            // pending maps raft proposal index to the number of pending mutations for this proposal.
            let pending: RefCell<HashMap<u64, u64>> = RefCell::new(HashMap::new());
            let waiters: RefCell<HashMap<u64, Vec<Sender<()>>>> = RefCell::new(HashMap::new());

            let get_delta = |done: bool| {
                if done {
                    0u64
                } else {
                    1
                }
            };

            let mut process_one = |idx: u64, done: bool| {
                // If not already done, then set. Otherwise, don't undo a done entry.
                let mut pending = pending.borrow_mut();
                let mut waiters = waiters.borrow_mut();

                match pending.entry(idx) {
                    std::collections::hash_map::Entry::Occupied(mut ent) => {
                        let delta = get_delta(done);
                        ent.get_mut().add_assign(delta);

                        // Update mark by going through all indices in order; and checking if they have
                        // been done. Stop at the first index, which isn't done.
                        let done_until = self.data.done_until();
                        assert!(
                            done_until <= idx,
                            "name: {}, done_until: {}, idx: {}",
                            self.data.name,
                            done_until,
                            idx
                        );

                        let mut until = done_until;
                        while !heap.is_empty() {
                            let min = heap.peek().unwrap().0;
                            if let Some(done) = pending.get(&min) {
                                if done.gt(&0) {
                                    break; // len(heap) will be > 0.
                                }
                            }
                            // Even if done is called multiple times causing it to become
                            // negative, we should still pop the index.
                            heap.pop();
                            pending.remove(&min);
                            until = min;
                        }

                        if until != done_until {
                            assert_eq!(
                                self.data.done_until.compare_exchange(
                                    done_until,
                                    until,
                                    Ordering::SeqCst,
                                    Ordering::Acquire
                                ),
                                Ok(done_until)
                            );
                        }

                        if until - done_until <= waiters.len() as u64 {
                            // Close channel and remove from waiters.
                            for idx in done_until + 1..=until {
                                if let Some(wait_tx) = waiters.remove(&idx) {
                                    drop(wait_tx);
                                }
                            }
                        } else {
                            // Close and drop idx <= util channels.
                            waiters.retain(|idx, _| *idx > until);
                        }
                    }
                    std::collections::hash_map::Entry::Vacant(ent) => {
                        heap.push(Reverse(idx));
                        let delta = get_delta(done);
                        ent.insert(delta);

                        // Update mark by going through all indices in order; and checking if they have
                        // been done. Stop at the first index, which isn't done.
                        let done_until = self.data.done_until();
                        assert!(
                            done_until <= idx,
                            "name: {}, done_until: {}, idx: {}",
                            self.data.name,
                            done_until,
                            idx
                        );

                        let mut until = done_until;

                        while !heap.is_empty() {
                            let min = heap.peek().unwrap().0;
                            if let Some(done) = pending.get(&min) {
                                if done.gt(&0) {
                                    break; // len(heap) will be > 0.
                                }
                            }
                            // Even if done is called multiple times causing it to become
                            // negative, we should still pop the index.
                            heap.pop();
                            pending.remove(&min);
                            until = min;
                        }

                        if until != done_until {
                            assert_eq!(
                                self.data.done_until.compare_exchange(
                                    done_until,
                                    until,
                                    Ordering::SeqCst,
                                    Ordering::Acquire
                                ),
                                Ok(done_until)
                            );
                        }

                        if until - done_until <= waiters.len() as u64 {
                            // Close channel and remove from waiters.
                            for idx in done_until + 1..=until {
                                if let Some(wait_tx) = waiters.remove(&idx) {
                                    drop(wait_tx);
                                }
                            }
                        } else {
                            // Close and drop idx <= util channels.
                            waiters.retain(|idx, _| *idx > until);
                        }
                    }
                }
            };

            loop {
                select! {
                    recv(closer.has_been_closed()) -> _ => return,
                    recv(self.data.mark_rx) -> mark => {
                        match mark {
                            Ok(mark) => {
                                if let Some(wait_tx) = mark.waiter_tx {
                                    if let Index::Single(index) = mark.index {
                                        let done_until = self.data.done_until.load(Ordering::SeqCst);
                                        if done_until >= index {
                                            let _ = wait_tx; // Close channel.
                                        } else{
                                            let mut waiters = waiters.borrow_mut();
                                            if let Some(ws) = waiters.get_mut(&index) {
                                                ws.push(wait_tx);
                                            } else {
                                                waiters.insert(index, vec![wait_tx]);
                                            }
                                        }
                                    } else {
                                        panic!("waiter used without index");
                                    }
                                } else {
                                    match mark.index {
                                        Index::Single(idx) => {
                                            process_one(idx, mark.done);
                                        },
                                        Index::Multiple(indices) => {
                                            for idx in indices {
                                                process_one(idx, mark.done);
                                            }
                                        },
                                    }
                                }
                            },
                            Err(_) => {
                                // Channel closed.
                                #[cfg(feature = "tracing")]
                                {
                                    tracing::error!(target: "watermark", err = "watermark has been dropped.");
                                }
                                return;
                            }
                        }
                    }
                }
            }
        });
    }
}
