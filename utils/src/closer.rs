use core::sync::atomic::{AtomicBool, Ordering};

use super::ref_counter::RefCounter;
use arc_swap::ArcSwapOption;
use crossbeam_channel::{unbounded, Receiver, Sender};
use wg::WaitGroup;

pub struct Canceled;

#[derive(Debug)]
struct Canceler {
    tx: ArcSwapOption<Sender<()>>,
    err: AtomicBool,
}

impl Canceler {
    #[inline]
    fn cancel(&self) {
        let canceled = self.err.load(Ordering::Acquire);
        if canceled {
            return;
        }

        self.err.store(true, Ordering::Release);
        self.tx.swap(None);
    }
}

#[derive(Debug)]
#[repr(transparent)]
struct CancelContext {
    rx: Receiver<()>,
}

impl CancelContext {
    fn new() -> (Self, Canceler) {
        let (tx, rx) = unbounded();
        (
            Self { rx },
            Canceler {
                tx: ArcSwapOption::from_pointee(tx),
                err: AtomicBool::new(false),
            },
        )
    }

    #[inline]
    fn done(&self) -> &Receiver<()> {
        &self.rx
    }
}

/// Closer holds the two things we need to close a thread and wait for it to
/// finish: a chan to tell the thread to shut down, and a WaitGroup with
/// which to wait for it to finish shutting down.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Closer {
    inner: RefCounter<CloserInner>,
}

#[derive(Debug)]
struct CloserInner {
    wg: WaitGroup,
    ctx: CancelContext,
    cancel: Canceler,
}

impl CloserInner {
    #[inline]
    fn new() -> Self {
        let (ctx, cancel) = CancelContext::new();
        Self {
            wg: WaitGroup::new(),
            ctx,
            cancel,
        }
    }

    #[inline]
    fn new_with_initial(initial: usize) -> Self {
        let (ctx, cancel) = CancelContext::new();
        Self {
            wg: WaitGroup::new().add(initial),
            ctx,
            cancel,
        }
    }
}

impl Default for Closer {
    fn default() -> Self {
        Self {
            inner: RefCounter::new(CloserInner::new()),
        }
    }
}

impl Closer {
    /// Constructs a new [`Closer`], with an initial count on the [`WaitGroup`].
    #[inline]
    pub fn new(initial: usize) -> Self {
        Self {
            inner: RefCounter::new(CloserInner::new_with_initial(initial)),
        }
    }

    /// Adds delta to the [`WaitGroup`].
    #[inline]
    pub fn add_running(&self, running: usize) {
        self.inner.wg.add(running);
    }

    /// Calls [`WaitGroup::done`] on the [`WaitGroup`].
    #[inline]
    pub fn done(&self) {
        self.inner.wg.done();
    }

    /// Signals the [`Closer::has_been_closed`] signal.
    #[inline]
    pub fn signal(&self) {
        self.inner.cancel.cancel();
    }

    /// Gets signaled when [`Closer::signal`] is called.
    #[inline]
    pub fn has_been_closed(&self) -> &Receiver<()> {
        self.inner.ctx.done()
    }

    /// Waits on the [`WaitGroup`]. (It waits for the Closer's initial value, [`Closer::add_running`], and [`Closer::done`]
    /// calls to balance out.)
    #[inline]
    pub fn wait(&self) {
        self.inner.wg.wait();
    }

    /// Calls [`Closer::signal`], then [`Closer::wait`].
    #[inline]
    pub fn signal_and_wait(&self) {
        self.signal();
        self.wait();
    }
}

#[test]
fn test_multiple_singles() {
    let closer = Closer::default();
    closer.signal();
    closer.signal();
    closer.signal_and_wait();

    let closer = Closer::new(1);
    closer.done();
    closer.signal_and_wait();
    closer.signal_and_wait();
    closer.signal();
}

#[test]
fn test_closer() {
    let closer = Closer::new(1);
    let tc = closer.clone();
    std::thread::spawn(move || {
        if let Err(err) = tc.has_been_closed().recv() {
            eprintln!("err: {}", err);
        }
        tc.done();
    });
    closer.signal_and_wait();
}

#[test]
fn test_closer_() {
    use core::time::Duration;

    let (tx, rx) = unbounded();

    let c = Closer::default();

    for _ in 0..10 {
        let c = c.clone();
        let tx = tx.clone();
        std::thread::spawn(move || {
            assert!(c.has_been_closed().recv().is_err());
            tx.send(()).unwrap();
        });
    }
    c.signal();
    for _ in 0..10 {
        rx.recv_timeout(Duration::from_millis(1000)).unwrap();
    }
}

#[test]
fn test_processor() {}
