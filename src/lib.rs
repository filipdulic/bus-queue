extern crate arc_swap;
#[allow(unused_imports)]
use std::sync::mpsc::{RecvError, RecvTimeoutError, SendError, TryRecvError, TrySendError};
pub mod sync;
#[cfg(feature = "async")]
extern crate futures;
#[cfg(feature = "async")]
pub mod async;
