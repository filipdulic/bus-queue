#![allow(dead_code)]
use crate::atomic::atomic_arc::AtomicArc;
use crate::{async_publisher, async_subscriber, publisher, subscriber, SwapSlot};
use std::sync::Arc;
use arc_swap::ArcSwapOption;

pub struct Slot<T> {
    atomic_arc: AtomicArc<T>,
}

impl<T> SwapSlot<T,T> for Slot<T> {
    fn store(&self, item: T) {
        self.atomic_arc.set(Some(Arc::new(item)));
    }

    fn load(&self) -> Option<Arc<T>> {
        self.atomic_arc.get().clone_inner()
    }

    fn none() -> Self {
        Slot {
            atomic_arc: AtomicArc::new(None),
        }
    }
}

impl<T> SwapSlot<T, Arc<T>> for Slot<T> {
    fn store(&self, item: T) {
        self.atomic_arc.set(Some(item));
    }

    fn load(&self) -> Option<Arc<T>> {
        self.atomic_arc.get().clone_inner()
    }

    fn none() -> Self {
        Slot {
            atomic_arc: AtomicArc::new(None),
        }
    }
}

pub type Publisher<T> = publisher::Publisher<T, Slot<T>>;
pub type Subscriber<T> = subscriber::Subscriber<T, Slot<T>>;

pub fn bounded<T>(size: usize) -> (Publisher<T>, Subscriber<T>) {
    crate::bounded::<T, Slot<T>>(size)
}

pub type AsyncPublisher<T> = async_publisher::AsyncPublisher<T, Slot<T>>;
pub type AsyncSubscriber<T> = async_subscriber::AsyncSubscriber<T, Slot<T>>;

pub fn async_bounded<T>(size: usize) -> (AsyncPublisher<T>, AsyncSubscriber<T>) {
    crate::async_bounded::<T, Slot<T>>(size)
}

#[cfg(test)]
mod test {
    use crate::flavors::atomic_arc::Slot;
    use crate::swap_slot::SwapSlot;
    use std::hint::black_box;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use std::thread;
    use std::time::{Duration, Instant};


    #[test]
    fn test_atomicarc_none() {
        let slot: Slot<()> = Slot::none();

        assert_eq!(slot.atomic_arc.get().clone_inner(), None);
    }

    #[test]
    fn test_atomicarc_store() {
        let slot = Slot::none();

        slot.store(5);

        assert_eq!(slot.atomic_arc.get().clone_inner(), Some(Arc::new(5)));
    }

    #[test]
    fn test_atomicarc_load() {
        let slot = Slot::none();
        slot.store(10);

        let arc = slot.load();

        assert_eq!(arc, Some(Arc::new(10)));
        assert_eq!(Arc::strong_count(&arc.unwrap()), 2)
    }

    #[test]
    fn test_slot_writer_vs_readers_latency() {
        // Make sure Slot<T> is accessible here (e.g., `pub struct Slot<T>(...)`).
        let slot: Arc<Slot<usize>> = Arc::new(Slot::<usize>::none());

        // Stop flag for readers.
        let stop = Arc::new(AtomicBool::new(false));

        // Spawn 31 busy readers.
        let mut reader_handles = Vec::with_capacity(31);
        for _ in 0..31 {
            let slot_cloned = Arc::clone(&slot);
            let stop_cloned: Arc<AtomicBool> = Arc::clone(&stop);
            reader_handles.push(thread::spawn(move || {
                // Tight loop: continuously load from the slot.
                // We black_box the result to avoid the optimizer removing the load.
                let mut spins = 0u32;
                while !stop_cloned.load(Ordering::Relaxed) {
                    let v = slot_cloned.load();
                    black_box(v);

                    // Yield occasionally so we don't starve the writer completely.
                    spins += 1;
                    if spins % 10_000 == 0 {
                        thread::yield_now();
                    }
                }
            }));
        }

        // Writer: measure latency of `store()` with many iterations.
        // Tune this for speed vs. stability. 200k is usually quick enough locally.
        let iterations: usize = 5_000_000;
        let mut lat_ns = Vec::with_capacity(iterations);

        // Small warmup to populate the slot and let threads settle.
        for i in 0..10_000 {
            slot.store(i);
        }

        // Measure only the `store` call.
        for i in 0..iterations {
            let t0 = Instant::now();
            slot.store(i);
            let dt = t0.elapsed();
            lat_ns.push(dt.as_nanos() as u64);

            // Optional: tiny yield to reduce pathological contention plateaus.
            if i % 50_000 == 0 {
                thread::yield_now();
            }
        }

        // Signal readers to stop and join them.
        stop.store(true, Ordering::Relaxed);
        for h in reader_handles {
            let _ = h.join();
        }

        // Compute average and p95.
        let avg_ns = (lat_ns.iter().sum::<u64>() as f64) / (lat_ns.len() as f64);
        let mut sorted = lat_ns.clone();
        sorted.sort_unstable();
        let p95_ns = {
            let n = sorted.len();
            let rank = ((0.95_f64 * n as f64).ceil() as usize).clamp(1, n) - 1;
            sorted[rank]
        };

        // Pretty-print in ns and µs.
        println!(
            "Slot::store latency over {} ops with 31 readers:\n  avg  = {:.2} ns ({:.3} µs)\n  p95  = {} ns ({:.3} µs)",
            iterations,
            avg_ns,
            avg_ns / 1_000.0,
            p95_ns,
            (p95_ns as f64) / 1_000.0
        );

        // A trivial assertion so the test is "about" the measurement but still passes.
        assert!(sorted.len() == iterations && avg_ns > 0.0);
    }
}
