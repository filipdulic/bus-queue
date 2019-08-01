//! # Lock-free Bounded Non-Blocking Pub-Sub Queue
//!
//! This is a publish subscribe pattern queue, where the publisher is never blocked by
//! slow subscribers. The side effect is that slow subscribers will miss messages. The intended
//! use-case are high throughput streams where receiving the latest message is prioritized over
//! receiving the entire stream. Market Data Feeds, Live Streams, etc....
//!
//! The underlying data-structure is a vector of Arc(s) eliminating the use of copies.
//!
//!## Features
//! * Lock-Free Write/Read - Lock-Free for Publisher and Lock-Free for Subscribers.
//! * Bounded - Constant size of memory used, max is `sizeof(MsgObject)*(queue_size + sub_cnt + 1)`.
//!   This is an edge-case where each subscriber is holding a ref to an object while the publisher
//!   has published a full length of queue in the mean time.
//! * Non-Blocking - The queue never blocks the publisher, slow subscribers miss data proportinal to
//!   their speed.
//! * Pub-Sub - Every Subscriber that can keep up with the Publisher will recieve all the data the
//!   Publisher publishes.
//! * [`sync`]/[`async`] - both interfaces are provided, as well as a bare queue implementation
//!   without the thread synchronisation ,and futures logic.
//! * std::sync::mpsc like interface - The API is modeled after the standard library mpsc queue,
//!   channel function are used to create a tuple of (Publisher, Subscriber), while the Clone trait
//!   on Subscriber creates additional subscribers to the same Publisher.
//!
//! [`sync::Publisher`], [`async::Publisher`], and [`BarePublisher`] are used to broadcast data to
//! [`sync::Subscriber`], [`async::Subscriber`], and [`BareSubscriber`] pools. Subscribers are
//! clone-able such that many threads, or futures, can receive data simultaneously. The only
//! limitation is that Subscribers have to keep up with the frequency of the Publisher. If a
//! Subscriber is slow it will drop data.
//!
//! ## Disconnection
//!
//! The broadcast and receive operations on channels will all return a [`Result`]
//! indicating whether the operation succeeded or not. An unsuccessful operation
//! is normally indicative of the other half of a channel having "hung up" by
//! being dropped in its corresponding thread.
//!
//! Once half of a channel has been deallocated, most operations can no longer
//! continue to make progress, so [`Err`] will be returned. Many applications
//! will continue to [`unwrap`] the results returned from this module,
//! instigating a propagation of failure among threads if one unexpectedly dies.
//!
//!
//! # Examples
//! ## Simple bare usage
//! ```rust
//! extern crate bus_queue;
//!
//! use bus_queue::bare_channel;
//!
//! fn main() {
//!     let (tx, rx) = bare_channel(10);
//!     (1..15).for_each(|x| tx.broadcast(x).unwrap());
//!
//!     let received: Vec<i32> = rx.map(|x| *x).collect();
//!     // Test that only the last 10 elements are in the received list.
//!     let expected: Vec<i32> = (5..15).collect();
//!
//!     assert_eq!(expected, received);
//! }
//! ```
//! ## Simple synchronous usage
//! ```rust
//! extern crate bus_queue;
//!
//! use bus_queue::sync;
//! fn main() {
//!     // Create a sync channel
//!     let (mut tx, rx) = sync::channel(10);
//!     // spawn tx thread, broadcast all and drop publisher.
//!     let tx_t = std::thread::spawn(move || {
//!         (1..15).for_each(|x| tx.broadcast(x).unwrap());
//!     });
//!     // small sleep for the tx thread to send and close, before rx thread is called
//!     std::thread::sleep(std::time::Duration::from_millis(100));
//!
//!     // spawn rx thread to get all the items left in the buffer
//!     let rx_t = std::thread::spawn(move || {
//!         let received: Vec<i32> = rx.map(|x| *x).collect();
//!         // Test that only the last 10 elements are in the received list.
//!         let expected: Vec<i32> = (5..15).collect();
//!         assert_eq!(received, expected);
//!     });
//!
//!     tx_t.join().unwrap();
//!     rx_t.join().unwrap();
//! }
//! ```
//! ## Simple asynchronous usage
//! ```rust
//! extern crate bus_queue;
//! extern crate futures;
//! extern crate tokio;
//!
//! use bus_queue::async_;
//! use futures::*;
//! use tokio::runtime::current_thread::Runtime;
//!
//! fn main() {
//!     let mut rt = Runtime::new().unwrap();
//!     let (tx, rx) = async_::channel(10);
//!     let sent: Vec<i32> = (1..15).collect();
//!     let publisher = stream::iter_ok(sent)
//!         .forward(tx)
//!         .and_then(|(_, mut sink)| sink.close())
//!         .map_err(|_| ())
//!         .map(|_| ());
//!
//!     rt.spawn(publisher);
//!
//!     let received: Vec<i32> = rt.block_on(rx.map(|x| *x).collect()).unwrap();
//!     // Test that only the last 10 elements are in the received list.
//!     let expected: Vec<i32> = (5..15).collect();
//!     assert_eq!(expected, received);
//! }
//! ```
//!
//! [`BarePublisher`]: struct.BarePublisher.html
//! [`BareSubscriber`]: struct.BareSubscriber.html
//! [`sync`]: sync/index.html
//! [`async`]: async/index.html
//! [`sync::Publisher`]: sync/struct.Publisher.html
//! [`sync::Subscriber`]: sync/struct.Subscriber.html
//! [`async::Publisher`]: async/struct.Publisher.html
//! [`async::Subscriber`]: async/struct.Subscriber.html
//! [`Result`]: ../../../std/result/enum.Result.html
//! [`Err`]: ../../../std/result/enum.Result.html#variant.Err
//! [`unwrap`]: ../../../std/result/enum.Result.html#method.unwrap
extern crate arc_swap;
extern crate lockfree;
use arc_swap::{ArcSwap, ArcSwapOption};
use std::fmt;
use std::iter::Iterator;
pub use std::sync::mpsc::{RecvError, RecvTimeoutError, SendError, TryRecvError};
use std::sync::{atomic::AtomicBool, atomic::AtomicUsize, atomic::Ordering, Arc};

struct AtomicCounter {
    count: AtomicUsize,
}

impl AtomicCounter {
    fn new(c: usize) -> Self {
        AtomicCounter {
            count: AtomicUsize::new(c),
        }
    }
    #[inline(always)]
    fn get(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }
    #[inline(always)]
    fn set(&self, val: usize) {
        self.count.store(val, Ordering::Release);
    }
    #[inline(always)]
    fn inc(&self) {
        self.count.fetch_add(1, Ordering::AcqRel);
    }
    #[inline(always)]
    fn dec(&self) {
        self.count.fetch_sub(1, Ordering::AcqRel);
    }
}

impl fmt::Debug for AtomicCounter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AtomicCounter: {}", self.get())
    }
}

impl PartialEq for AtomicCounter {
    fn eq(&self, other: &AtomicCounter) -> bool {
        self.count.load(Ordering::AcqRel) == other.count.load(Ordering::AcqRel)
    }
}

impl Eq for AtomicCounter {}

/// Bare implementation of the publisher.
#[derive(Debug)]
pub struct BarePublisher<T: Send> {
    buffer: Arc<Vec<ArcSwapOption<T>>>,
    size: usize,
    wi: Arc<AtomicCounter>,
    sub_cnt: Arc<AtomicCounter>,
    is_pub_available: Arc<AtomicBool>,
}

/// Bare implementation of the subscriber.
#[derive(Debug)]
pub struct BareSubscriber<T: Send> {
    buffer: Arc<Vec<ArcSwapOption<T>>>,
    wi: Arc<AtomicCounter>,
    ri: AtomicCounter,
    size: usize,
    sub_cnt: Arc<AtomicCounter>,
    is_pub_available: Arc<AtomicBool>,
}

pub trait GetSubCount {
    fn get_sub_count(&self) -> usize;
}
/// Function used to create and initialise a ( BarePublisher, BareSubscriber ) tuple.
pub fn bare_channel<T: Send>(size: usize) -> (BarePublisher<T>, BareSubscriber<T>) {
    let size = size + 1;
    let mut buffer = Vec::new();
    buffer.resize(size, ArcSwapOption::new(None));
    let buffer = Arc::new(buffer);
    let sub_cnt = Arc::new(AtomicCounter::new(1));
    let wi = Arc::new(AtomicCounter::new(0));
    let is_pub_available = Arc::new(AtomicBool::new(true));
    (
        BarePublisher {
            buffer: buffer.clone(),
            size,
            wi: wi.clone(),
            sub_cnt: sub_cnt.clone(),
            is_pub_available: is_pub_available.clone(),
        },
        BareSubscriber {
            buffer,
            size,
            wi,
            ri: AtomicCounter::new(0),
            sub_cnt,
            is_pub_available,
        },
    )
}

impl<T: Send> BarePublisher<T> {
    /// Publishes values to the circular buffer at wi % size
    /// # Arguments
    /// * `object` - owned object to be published

    pub fn broadcast(&self, object: T) -> Result<(), SendError<T>> {
        if self.sub_cnt.get() == 0 {
            return Err(SendError(object));
        }
        self.buffer[self.wi.get() % self.size].store(Some(Arc::new(object)));
        self.wi.inc();
        Ok(())
    }
}

impl<T: Send> GetSubCount for BarePublisher<T> {
    fn get_sub_count(&self) -> usize {
        self.sub_cnt.get()
    }
}
/// Drop trait is used to let subscribers know that publisher is no longer available.
impl<T: Send> Drop for BarePublisher<T> {
    fn drop(&mut self) {
        self.is_pub_available.store(false, Ordering::Relaxed);
    }
}

impl<T: Send> PartialEq for BarePublisher<T> {
    fn eq(&self, other: &BarePublisher<T>) -> bool {
        Arc::ptr_eq(&self.buffer, &other.buffer)
    }
}

impl<T: Send> Eq for BarePublisher<T> {}

impl<T: Send> BareSubscriber<T> {
    /// Receives some atomic reference to an object if queue is not empty, or None if it is. Never
    /// Blocks
    fn is_pub_available(&self) -> bool {
        self.is_pub_available.load(Ordering::Relaxed)
    }

    pub fn try_recv(&self) -> Result<Arc<T>, TryRecvError> {
        if self.ri.get() == self.wi.get() {
            if self.is_pub_available() {
                return Err(TryRecvError::Empty);
            } else {
                return Err(TryRecvError::Disconnected);
            }
        }

        loop {
            let val = self.buffer[self.ri.get() % self.size].load().unwrap();

            if self.wi.get() >= self.ri.get() + self.size {
                self.ri.set(self.wi.get() - self.size + 1);
            } else {
                self.ri.inc();
                return Ok(val);
            }
        }
    }
}

/// Clone trait is used to create another BareSubscriber object, subscribed to the same
/// Publisher the initial object was subscribed to.
impl<T: Send> Clone for BareSubscriber<T> {
    fn clone(&self) -> Self {
        self.sub_cnt.inc();
        Self {
            buffer: self.buffer.clone(),
            wi: self.wi.clone(),
            ri: AtomicCounter::new(self.ri.get()),
            size: self.size,
            sub_cnt: Arc::new(AtomicCounter::new(self.sub_cnt.get())),
            is_pub_available: self.is_pub_available.clone(),
        }
    }
}

impl<T: Send> Drop for BareSubscriber<T> {
    fn drop(&mut self) {
        self.sub_cnt.dec();
    }
}

impl<T: Send> PartialEq for BareSubscriber<T> {
    fn eq(&self, other: &BareSubscriber<T>) -> bool {
        Arc::ptr_eq(&self.buffer, &other.buffer) && self.ri == other.ri
    }
}

impl<T: Send> Eq for BareSubscriber<T> {}

impl<T: Send> Iterator for BareSubscriber<T> {
    type Item = Arc<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.try_recv().ok()
    }
}

/// Helper struct used by sync and async implementations to wake Tasks / Threads
#[derive(Debug)]
pub struct Waker<T> {
    /// Vector of Tasks / Threads to be woken up.
    pub sleepers: Vec<Arc<T>>,
    /// A mpsc Receiver used to receive Tasks / Threads to be registered.
    receiver: lockfree::channel::mpsc::Receiver<Arc<T>>,
}

/// Helper struct used by sync and async implementations to register Tasks / Threads to
/// be woken up.
#[derive(Debug)]
pub struct Sleeper<T> {
    /// Current Task / Thread to be woken up.
    pub sleeper: Arc<T>,
    /// mpsc Sender used to register Task / Thread.
    pub sender: lockfree::channel::mpsc::Sender<Arc<T>>,
}

impl<T> Waker<T> {
    /// Register all the Tasks / Threads sent for registration.
    pub fn register_receivers(&mut self) {
        for receiver in self.receiver.recv() {
            self.sleepers.push(receiver);
        }
    }
}

/// Function used to create a ( Waker, Sleeper ) tuple.
pub fn alarm<T>(current: T) -> (Waker<T>, Sleeper<T>) {
    let mut vec = Vec::new();
    let (sender, receiver) = lockfree::channel::mpsc::create();
    let arc_t = Arc::new(current);
    vec.push(arc_t.clone());
    (
        Waker {
            sleepers: vec,
            receiver,
        },
        Sleeper {
            sleeper: arc_t,
            sender,
        },
    )
}

pub mod sync;
#[cfg(feature = "async")]
extern crate futures;
#[cfg(feature = "async")]
pub mod async_;
#[cfg(feature = "async")]
#[deprecated(
    since = "0.3.8",
    note = "Renamed to async_ to avoid conflict with keyword."
)]
pub mod async {
    pub use super::async_::*;
}
