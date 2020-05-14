#![allow(dead_code)]
use crate::{async_publisher, async_subscriber, publisher, subscriber, SwapSlot};
use std::sync::{Arc, RwLock};

pub type Slot<T> = RwSlot<T>;
type RwSlot<T> = RwLock<Option<Arc<T>>>;

impl<T> SwapSlot<T> for RwSlot<T> {
    fn store(&self, item: T) {
        *self.write().unwrap() = Some(Arc::new(item));
    }

    fn load(&self) -> Option<Arc<T>> {
        self.read().unwrap().clone()
    }

    fn none() -> Self {
        Self::new(None)
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
    use crate::flavors::rw_lock::RwSlot;
    use crate::swap_slot::SwapSlot;
    use std::sync::Arc;

    #[test]
    fn test_rwslot_none() {
        let item: RwSlot<i32> = RwSlot::none();

        assert_eq!(item.read().unwrap().clone(), None);
    }

    #[test]
    fn test_rwslot_store() {
        let item = RwSlot::none();

        SwapSlot::store(&item, 5);

        assert_eq!(item.read().unwrap().clone(), Some(Arc::new(5)));
    }

    #[test]
    fn test_rwslot_load() {
        let item = RwSlot::none();
        SwapSlot::store(&item, 10);

        let arc = SwapSlot::load(&item);

        assert_eq!(arc, Some(Arc::new(10)));
        assert_eq!(Arc::strong_count(&arc.unwrap()), 2)
    }
}
