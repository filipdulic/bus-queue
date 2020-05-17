#![allow(dead_code)]
use crate::atomic::atomic_arc::AtomicArc;
use crate::{async_publisher, async_subscriber, publisher, subscriber, SwapSlot};
use std::sync::Arc;

pub struct Slot<T> {
    atomic_arc: AtomicArc<T>,
}

impl<T> SwapSlot<T> for Slot<T> {
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
    use std::sync::Arc;

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
}
