#![allow(dead_code)]
use crate::{async_publisher, async_subscriber, publisher, subscriber, SwapSlot};
use arc_swap::ArcSwapOption;
use std::sync::Arc;

pub struct Slot<T> {
    shared: ArcSwapOption<T>,
}

impl<T> SwapSlot<T, T> for Slot<T> {
    fn store(&self, item: T) {
        self.shared.store(Some(Arc::new(item)))
    }

    fn load(&self) -> Option<Arc<T>> {
        self.shared.load_full()
    }

    fn none() -> Self {
        Slot {
            shared: ArcSwapOption::new(None),
        }
    }
}

impl<T> SwapSlot<T, Arc<T>> for Slot<T> {
    fn store(&self, item: Arc<T>) {
        self.shared.store(Some(item))
    }
    fn load(&self) -> Option<Arc<T>> {
        self.shared.load_full()
    }
    fn none() -> Self {
        Slot {
            shared: ArcSwapOption::new(None),
        }
    }
}

pub type Publisher<T, I> = publisher::Publisher<T, I, Slot<T>>;
pub type Subscriber<T, I> = subscriber::Subscriber<T, I, Slot<T>>;

pub fn bounded<T>(size: usize) -> (Publisher<T, T>, Subscriber<T, T>) {
    crate::bounded::<T, T, Slot<T>>(size)
}

pub fn bounded_arc<T>(size: usize) -> (Publisher<T, Arc<T>>, Subscriber<T, Arc<T>>) {
    crate::bounded::<T, Arc<T>, Slot<T>>(size)
}

pub type AsyncPublisher<T, I> = async_publisher::AsyncPublisher<T, I, Slot<T>>;
pub type AsyncSubscriber<T, I> = async_subscriber::AsyncSubscriber<T, I, Slot<T>>;

pub fn async_bounded<T>(size: usize) -> (AsyncPublisher<T, T>, AsyncSubscriber<T, T>) {
    crate::async_bounded::<T,T, Slot<T>>(size)
}

pub fn async_bounded_arc<T>(size: usize) -> (AsyncPublisher<T,Arc<T>>, AsyncSubscriber<T, Arc<T>>) {
    crate::async_bounded::<T, Arc<T>, Slot<T>>(size)
}

#[cfg(test)]
mod test {
    use crate::flavors::arc_swap::Slot;
    use crate::swap_slot::SwapSlot;
    use std::sync::Arc;
    use std::hint::black_box;
    use std::ops::Deref;
    use std::sync::{
        atomic::{AtomicBool, Ordering},    };
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn test_archswap_none() {
        let slot: Slot<usize> = SwapSlot::<usize, usize>::none();

        assert_eq!(slot.shared.load_full(), None);
    }

    #[test]
    fn test_archswap_store() {
        let slot: Slot<usize> = SwapSlot::<usize, usize>::none();

        slot.store(5);

        assert_eq!(slot.shared.load_full(), Some(Arc::new(5)));
    }

    #[test]
    fn test_archswap_load() {
        let slot: Slot<i32> = SwapSlot::<i32, i32>::none();
        slot.store(10);

        let arc = SwapSlot::<i32, i32>::load(&slot);

        assert_eq!(arc, Some(Arc::new(10)));
        assert_eq!(Arc::strong_count(&arc.unwrap()), 2)
    }
    #[test]
    fn test_slot_writer_vs_readers_latency() {
        // Make sure Slot<T> is accessible here (e.g., `pub struct Slot<T>(...)`).
        let slot: Slot<usize> = SwapSlot::<usize, usize>::none();
        let arc_slot: Arc<Slot<usize>> = Arc::new(slot);

        // Stop flag for readers.
        let stop = Arc::new(AtomicBool::new(false));

        // Spawn 31 busy readers.
        let mut reader_handles = Vec::with_capacity(31);
        for _ in 0..31 {
            let slot_cloned = Arc::clone(&arc_slot);
            let stop_cloned: Arc<AtomicBool> = Arc::clone(&stop);
            reader_handles.push(thread::spawn(move || {
                // Tight loop: continuously load from the slot.
                // We black_box the result to avoid the optimizer removing the load.
                let mut spins = 0u32;
                while !stop_cloned.load(Ordering::Relaxed) {
                    let v = SwapSlot::<usize, usize>::load(slot_cloned.deref());
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
            arc_slot.store(i);
        }

        // Measure only the `store` call.
        for i in 0..iterations {
            let t0 = Instant::now();
            arc_slot.store(i);
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
