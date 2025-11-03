#![allow(dead_code)]
use crate::{SwapSlot, async_publisher, async_subscriber, publisher, subscriber};
use std::sync::{Arc, RwLock};

pub struct Slot<T> {
    lock: RwLock<Option<Arc<T>>>,
}

impl<T> SwapSlot<T> for Slot<T> {
    fn store(&self, item: impl Into<Arc<T>>) {
        *self.lock.write().unwrap() = Some(item.into());
    }

    fn load(&self) -> Option<Arc<T>> {
        self.lock.read().unwrap().clone()
    }

    fn none() -> Self {
        Slot {
            lock: RwLock::new(None),
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
    use crate::flavors::rw_lock::Slot;
    use crate::swap_slot::SwapSlot;
    use std::sync::Arc;

    #[test]
    fn test_rwslot_none() {
        let slot: Slot<()> = Slot::none();

        assert_eq!(slot.lock.read().unwrap().clone(), None);
    }

    #[test]
    fn test_rwslot_store() {
        let slot = Slot::none();

        slot.store(5);

        assert_eq!(slot.lock.read().unwrap().clone(), Some(Arc::new(5)));
    }

    #[test]
    fn test_rwslot_load() {
        let slot = Slot::none();
        slot.store(10);

        let arc = slot.load();

        assert_eq!(arc, Some(Arc::new(10)));
        assert_eq!(Arc::strong_count(&arc.unwrap()), 2)
    }
}

#[cfg(test)]
mod allocation_tests {
    use crate::flavors::allocation_tests::{allocs_current_thread, reset_allocs_current_thread};

    use super::*;

    #[test]
    fn store_with_arc_does_not_allocate_new_arc() {
        let slot = Slot::<u32>::none();
        let arc = Arc::new(123u32);

        // Ignore allocations from constructing `slot` and `arc`.
        reset_allocs_current_thread();

        // This should only move / clone the Arc; no new heap allocation for T.
        slot.store(arc.clone());

        // Might still be 0 or some tiny number depending on RwLock internals,
        // but definitely shouldn't be "one Arc allocation vs another" difference.
        let after = allocs_current_thread();
        assert_eq!(
            after, 0,
            "expected no additional allocations when storing an Arc"
        );
    }

    #[test]
    fn store_with_value_allocates_arc() {
        let slot = Slot::<u32>::none();

        reset_allocs_current_thread();

        // This goes through `impl From<T> for Arc<T>` and must allocate.
        slot.store(5u32);

        let after = allocs_current_thread();
        assert!(
            after == 1,
            "expected at least one allocation when storing a bare value T"
        );
    }
}
