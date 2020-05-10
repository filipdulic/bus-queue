#[cfg(feature = "arcswap")]
pub mod arc_swap;
#[cfg(feature = "rwlock")]
pub mod rw_lock;

#[cfg(feature = "atomic-arc")]
pub mod atomic_arc;

// #[cfg(feature = "conc-atomic")]
// pub mod conc_atomic {
//     use std::sync::Arc;
//     use conc::Atomic;
//     use crate::swap_slot::SwapSlot;
//     use crate::{channel, bus};
//     use std::sync::atomic::Ordering;
//     use std::ops::Deref;
//
//     impl<T> SwapSlot<T> for Atomic<T> {
//         fn store(&self, item: T) {
//             unsafe {
//                 let raw = Arc::into_raw(Arc::new(item)).as_raw();
//                 loop {
//                     let load = self.load_raw(Ordering::SeqCst);
//                     match self.compare_and_store_raw(load, raw,Ordering::SeqCst) {
//                         Ok(()) => {
//                             let arc = Arc::from_raw(load);
//                             return;
//                         },
//                         _ => {},
//                     }
//                 }
//             }
//         }
//
//         fn load(&self) -> Arc<T> {
//             unsafe {
//                 loop {
//                     let load = self.load_raw(Ordering::SeqCst);
//                     let arc = Arc::from_raw(load);
//                     let cloned = arc.clone();
//                     let raw = Arc::into_raw(arc).as_raw();
//                     match self.compare_and_store_raw(load, raw,Ordering::SeqCst) {
//                         Ok(()) => {
//                             return cloned;
//                         },
//                         _ => {
//                             std::mem::forget(cloned);
//                         },
//                     }
//                 }
//             }
//         }
//
//         fn none() -> Self {
//             Self::new(None)
//         }
//     }
//
//     pub type Sender<T> = channel::Sender<T, Atomic<T>>;
//     pub type Receiver<T> = channel::Receiver<T, Atomic<T>>;
//
//     pub fn raw_bounded<T>(size: usize) -> (Sender<T>, Receiver<T>) {
//         channel::bounded(size)
//     }
//
//     pub type Publisher<T> = bus::Publisher<T, Atomic<T>>;
//     pub type Subscriber<T> = bus::Subscriber<T, Atomic<T>>;
//
//     pub fn bounded<T>(size: usize) -> (Publisher<T>, Subscriber<T>) {
//         bus::bounded(size)
//     }
// }
