#![allow(dead_code)]
use crate::swap_slot::SwapSlot;
use crate::{bus, channel};
use std::sync::{Arc, RwLock};

pub type Slot<T> = RwSlot<T>;
type RwSlot<T> = RwLock<Option<Arc<T>>>;

impl<T> SwapSlot<T> for RwSlot<T> {
    fn store(&self, item: T) {
        *self.write().unwrap() = Some(Arc::new(item));
    }

    fn load(&self) -> Arc<T> {
        self.read().unwrap().as_ref().unwrap().clone()
    }

    fn none() -> Self {
        Self::new(None)
    }
}

pub type Sender<T> = channel::Sender<T, RwSlot<T>>;
pub type Receiver<T> = channel::Receiver<T, RwSlot<T>>;

pub fn raw_bounded<T>(size: usize) -> (Sender<T>, Receiver<T>) {
    channel::bounded::<T, RwSlot<T>>(size)
}

pub type Publisher<T> = bus::Publisher<T, RwSlot<T>>;
pub type Subscriber<T> = bus::Subscriber<T, RwSlot<T>>;

pub fn bounded<T>(size: usize) -> (Publisher<T>, Subscriber<T>) {
    bus::bounded::<T, RwSlot<T>>(size)
}

#[cfg(test)]
mod test {
    use crate::swap_slot::SwapSlot;
    use std::sync::Arc;

    #[test]
    fn test_rwlock() {
        use crate::flavors::rw_lock::RwSlot;
        let item = RwSlot::none();
        {
            let none = RwSlot::read(&item).unwrap();
            assert_eq!(none.as_ref(), None);
        }
        SwapSlot::store(&item, 1);
        let arc = SwapSlot::load(&item);
        assert_eq!(*arc, 1);
        assert_eq!(Arc::strong_count(&arc), 2);
        SwapSlot::store(&item, 1);
        assert_eq!(Arc::strong_count(&arc), 1);
    }
}
