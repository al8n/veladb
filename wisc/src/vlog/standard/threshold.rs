use bable::Histogram;
use core::sync::atomic::{AtomicI64, Ordering};
use crossbeam_channel::{bounded, select, Receiver, SendError, Sender};
use vela_utils::ref_counter::RefCounter;

pub struct ValueLogThreshold {
    percentile: f64,
    threshold: RefCounter<AtomicI64>,
    val_tx: Sender<Vec<i64>>,
    clear_tx: Sender<()>,
    close_tx: Sender<()>,
    metrics: RefCounter<Histogram>,
}

impl ValueLogThreshold {
    #[must_use]
    pub fn new(
        percentile: f64,
        max_value_threshold: f64,
        value_threshold: RefCounter<AtomicI64>,
    ) -> (Self, ValueLogThresholdProcessor) {
        let get_bounds = || {
            let (mxbd, mnbd) = (
                max_value_threshold,
                value_threshold.load(Ordering::SeqCst) as f64,
            );
            assert!(
                mxbd >= mnbd,
                "maximum threshold bound is less than the min threshold"
            );
            let size = (mxbd - mnbd + 1.0).min(1024.0);
            let bdstp = (mxbd - mnbd) / size;
            let bounds_size = size as usize;
            let mut bounds = Vec::with_capacity(bounds_size);
            for i in 0..bounds_size {
                if i == 0 {
                    bounds.push(mnbd);
                    continue;
                }

                if i == bounds_size - 1 {
                    bounds.push(mxbd);
                    continue;
                }

                bounds.push(bounds[i - 1] + bdstp);
            }

            bounds
        };

        let m = RefCounter::new(Histogram::new(get_bounds()));

        let (val_tx, val_rx) = bounded(1000);
        let (close_tx, close_rx) = bounded(1);
        let (clear_tx, clear_rx) = bounded(1);

        let this = Self {
            percentile,
            threshold: value_threshold.clone(),
            val_tx,
            clear_tx,
            close_tx,
            metrics: m.clone(),
        };

        let processor = ValueLogThresholdProcessor {
            percentile,
            threshold: value_threshold,
            metrics: m,
            close_rx,
            clear_rx,
            val_rx,
        };

        (this, processor)
    }

    #[inline]
    pub fn clear(&self, value_threshold: i64) -> Result<(), SendError<()>> {
        self.threshold.store(value_threshold, Ordering::SeqCst);
        self.clear_tx.send(())
    }

    #[inline]
    pub fn update(&self, sizes: Vec<i64>) -> Result<(), SendError<Vec<i64>>> {
        self.val_tx.send(sizes)
    }

    #[inline]
    pub fn close(&self) -> Result<(), SendError<()>> {
        self.close_tx.send(())
    }
}

pub struct ValueLogThresholdProcessor {
    percentile: f64,
    threshold: RefCounter<AtomicI64>,
    metrics: RefCounter<Histogram>,
    close_rx: Receiver<()>,
    clear_rx: Receiver<()>,
    val_rx: Receiver<Vec<i64>>,
}

impl ValueLogThresholdProcessor {
    pub fn spawn(self) {
        let Self {
            percentile,
            metrics,
            close_rx,
            clear_rx,
            val_rx,
            threshold,
        } = self;

        std::thread::spawn(move || {
            loop {
                select! {
                    recv(close_rx) -> _ => {
                        return;
                    }
                    recv(clear_rx) -> _ => {
                        metrics.clear();
                    }
                    recv(val_rx) -> val => {
                        if let Ok(val) = val {
                            for v in val {
                                metrics.update(v);
                            }

                            // we are making it to get Options.VlogPercentile so that values with sizes
                            // in range of Options.VlogPercentile will make it to the LSM tree and rest to the
                            // value log file.
                            let p = metrics.percentile(percentile) as i64;
                            if threshold.load(Ordering::Acquire) != p {

                                #[cfg(feature = "tracing")]
                                {
                                    tracing::info!(target: "vlog_threshold", "updating value of threshold to: {}", p);
                                }

                                threshold.store(p, Ordering::Release);
                            }

                        } else {
                            #[cfg(feature = "tracing")]
                            {
                                tracing::error!(target: "vlog_threshold",
                                "fail to receive value from channel");
                            }
                        }
                    }
                }
            }
        });
    }
}
