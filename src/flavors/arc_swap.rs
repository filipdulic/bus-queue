#![allow(dead_code)]
use crate::{async_publisher, async_subscriber, publisher, subscriber, SwapSlot};
use arc_swap::ArcSwapOption;
use std::sync::Arc;

pub type Slot<T> = ArcSwapOption<T>;

impl<T> SwapSlot<T> for ArcSwapOption<T> {
    fn store(&self, item: T) {
        self.store(Some(Arc::new(item)))
    }

    fn load(&self) -> Option<Arc<T>> {
        self.load_full()
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
    use crate::swap_slot::SwapSlot;
    use arc_swap::ArcSwapOption;
    use std::sync::Arc;

    #[test]
    fn test_archswap_none() {
        let item: ArcSwapOption<()> = ArcSwapOption::none();

        assert_eq!(item.load_full(), None);
    }

    #[test]
    fn test_archswap_store() {
        let item = ArcSwapOption::none();

        SwapSlot::store(&item, 5);

        assert_eq!(item.load_full(), Some(Arc::new(5)));
    }

    #[test]
    fn test_archswap_load() {
        let item = ArcSwapOption::none();
        SwapSlot::store(&item, 10);

        let arc = SwapSlot::load(&item);

        assert_eq!(arc, Some(Arc::new(10)));
        assert_eq!(Arc::strong_count(&arc.unwrap()), 2)
    }
}
