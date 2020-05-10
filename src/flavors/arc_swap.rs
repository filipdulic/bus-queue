use crate::bus;
use crate::channel;
use crate::swap_slot::SwapSlot;
use arc_swap::ArcSwapOption;
use std::sync::Arc;

pub type Slot<T> = ArcSwapOption<T>;

impl<T> SwapSlot<T> for ArcSwapOption<T> {
    fn store(&self, item: T) {
        self.store(Some(Arc::new(item)))
    }

    fn load(&self) -> Arc<T> {
        self.load_full().unwrap()
    }

    fn none() -> Self {
        Self::new(None)
    }
}
pub type Sender<T> = channel::Sender<T, ArcSwapOption<T>>;
pub type Receiver<T> = channel::Receiver<T, ArcSwapOption<T>>;

pub fn raw_bounded<T>(size: usize) -> (Sender<T>, Receiver<T>) {
    channel::bounded::<T, ArcSwapOption<T>>(size)
}

pub type Publisher<T> = bus::Publisher<T, ArcSwapOption<T>>;
pub type Subscriber<T> = bus::Subscriber<T, ArcSwapOption<T>>;

pub fn bounded<T>(size: usize) -> (Publisher<T>, Subscriber<T>) {
    bus::bounded::<T, ArcSwapOption<T>>(size)
}

#[cfg(test)]
mod test {
    use crate::swap_slot::SwapSlot;
    use std::sync::Arc;

    #[test]
    fn test_arcswap() {
        use arc_swap::ArcSwapOption;
        let item = ArcSwapOption::none();
        let none = ArcSwapOption::load_full(&item);
        assert_eq!(none, None);
        SwapSlot::store(&item, 1);
        let arc = SwapSlot::load(&item);
        assert_eq!(*arc, 1);
        assert_eq!(Arc::strong_count(&arc), 2);
        SwapSlot::store(&item, 1);
        assert_eq!(Arc::strong_count(&arc), 1);
    }
}
