#![allow(dead_code)]
use crate::atomic::atomic_arc::AtomicArc;
use crate::{
    async_bounded_queue, async_publisher, async_subscriber, bounded_queue, publisher, subscriber,
    SwapSlot,
};
use std::sync::Arc;

pub type Slot<T> = AtomicArc<T>;

impl<T> SwapSlot<T> for AtomicArc<T> {
    fn store(&self, item: T) {
        self.set(Some(Arc::new(item)));
    }

    fn load(&self) -> Option<Arc<T>> {
        self.get().clone_inner()
    }

    fn none() -> Self {
        AtomicArc::new(None)
    }
}

pub type Publisher<T> = publisher::GenericPublisher<T, Slot<T>>;
pub type Subscriber<T> = subscriber::GenericSubscriber<T, Slot<T>>;

pub fn raw_bounded<T>(size: usize) -> (Publisher<T>, Subscriber<T>) {
    bounded_queue::<T, Slot<T>>(size)
}

pub type AsyncPublisher<T> = async_publisher::GenericAsyncPublisher<T, Slot<T>>;
pub type AsyncSubscriber<T> = async_subscriber::GenericAsyncSubscriber<T, Slot<T>>;

pub fn bounded<T>(size: usize) -> (AsyncPublisher<T>, AsyncSubscriber<T>) {
    async_bounded_queue::<T, Slot<T>>(size)
}

#[cfg(test)]
mod test {
    use crate::atomic::atomic_arc::AtomicArc;
    use crate::swap_slot::SwapSlot;
    use std::sync::Arc;

    #[test]
    fn test_atomicarc_none() {
        let item: AtomicArc<i32> = AtomicArc::none();

        assert_eq!(item.get().clone_inner(), None);
    }

    #[test]
    fn test_atomicarc_store() {
        let item = AtomicArc::none();

        SwapSlot::store(&item, 5);

        assert_eq!(item.get().clone_inner(), Some(Arc::new(5)));
    }

    #[test]
    fn test_atomicarc_load() {
        let item = AtomicArc::none();
        SwapSlot::store(&item, 10);

        let arc = SwapSlot::load(&item);

        assert_eq!(arc, Some(Arc::new(10)));
        assert_eq!(Arc::strong_count(&arc.unwrap()), 2)
    }
}
