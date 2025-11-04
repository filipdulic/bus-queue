#![allow(dead_code)]
use crate::{SwapSlot, async_publisher, async_subscriber, publisher, subscriber};
use arc_swap::ArcSwapOption;
use std::sync::Arc;

pub struct Slot<T> {
    shared: ArcSwapOption<T>,
}

impl<T> SwapSlot<T> for Slot<T> {
    fn store(&self, item: impl Into<Arc<T>>) {
        self.shared.store(Some(item.into()));
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
    use crate::flavors::arc_swap::Slot;
    use crate::swap_slot::SwapSlot;
    use std::sync::Arc;

    #[test]
    fn test_archswap_none() {
        let slot: Slot<()> = Slot::none();

        assert_eq!(slot.shared.load_full(), None);
    }

    #[test]
    fn test_archswap_store() {
        let slot = Slot::none();

        slot.store(5);

        assert_eq!(slot.shared.load_full(), Some(Arc::new(5)));
    }

    #[test]
    fn test_archswap_load() {
        let slot = Slot::none();
        slot.store(10);

        let arc = slot.load();

        assert_eq!(arc, Some(Arc::new(10)));
        assert_eq!(Arc::strong_count(&arc.unwrap()), 2)
    }
}
