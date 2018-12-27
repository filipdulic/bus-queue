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
//!   channel function are used to create a tuple of (Publisher, Subscriber), while the Clone trait on Subscribre
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
//! ```
//! extern crate bus_queue;
//! use bus_queue::bare_channel;
//!
//!fn main() {
//!    let (mut tx, rx) = bare_channel(10);
//!    (1..15).for_each(|x| tx.broadcast(x).unwrap());
//!
//!    let received: Vec<i32> = rx.into_iter().map(|x| *x).collect();
//!    // Test that only the last 10 elements are in the received list.
//!    let expected: Vec<i32> = (5..15).collect();
//!
//!    assert_eq!(expected, received);
//!}
//! ```
//! ## Simple synchronous usage
//! ```
//! extern crate bus_queue;
//!
//! use bus_queue::sync;
//! use std::thread;
//! fn main() {
//!    // Create a sync channel
//!    let (mut tx, rx) = sync::channel(1);
//!    let t = thread::spawn(move|| {
//!        let received = rx.recv().unwrap();
//!        assert_eq!(*received, 10);
//!    });
//!    tx.broadcast(10).unwrap();
//!    t.join().unwrap();
//!}
//! ```
//! ## Simple asynchronous usage
//! ```
//! extern crate bus_queue;
//! extern crate futures;
//! extern crate tokio;
//!
//! use bus_queue::async;
//! use futures::future::Future;
//! use futures::*;
//! use tokio::runtime::Runtime;
//!
//! fn subscriber(rx: async::Subscriber<i32>) -> impl Future<Item = (), Error = ()> {
//!     assert_eq!(
//!         rx.map(|x| *x).collect().wait().unwrap(),
//!         vec![1, 2, 3, 4, 5]
//!     );
//!     future::ok(())
//! }
//!
//! fn main() {
//!     let mut rt = Runtime::new().unwrap();
//!     let (tx, rx): (async::Publisher<i32>, async::Subscriber<i32>) = async::channel(10);
//!
//!     let publisher = stream::iter_ok(vec![1, 2, 3, 4, 5])
//!         .forward(tx)
//!         .and_then(|(_, mut sink)| sink.close())
//!         .map_err(|_| ())
//!         .map(|_| ());
//!
//!     rt.spawn(publisher);
//!     rt.block_on(subscriber(rx)).unwrap();
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
use arc_swap::{ArcSwap, ArcSwapOption};
use std::iter::Iterator;
pub use std::sync::mpsc::{RecvError, RecvTimeoutError, SendError, TryRecvError};
use std::sync::{atomic::AtomicBool, atomic::AtomicUsize, atomic::Ordering, mpsc, Arc};

/// Bare implementation of the publisher.
#[derive(Debug)]
pub struct BarePublisher<T: Send> {
    buffer: Arc<Vec<ArcSwapOption<T>>>,
    wi: Arc<AtomicUsize>,
    size: usize,
    sub_cnt: Arc<AtomicUsize>,
    pub_available: Arc<AtomicBool>,
}
/// Bare implementation of the subscriber.
#[derive(Debug)]
pub struct BareSubscriber<T: Send> {
    buffer: Arc<Vec<ArcSwapOption<T>>>,
    wi: Arc<AtomicUsize>,
    ri: AtomicUsize,
    size: usize,
    sub_cnt: Arc<AtomicUsize>,
    pub_available: Arc<AtomicBool>,
}

/// Function used to create and initialise a ( BarePublisher, BareSubscriber ) tuple.
pub fn bare_channel<T: Send>(size: usize) -> (BarePublisher<T>, BareSubscriber<T>) {
    let size = size + 1;
    let mut buffer = Vec::new();
    buffer.resize(size, ArcSwapOption::new(None));
    let buffer = Arc::new(buffer);
    let sub_cnt = Arc::new(AtomicUsize::new(1));
    let wi = Arc::new(AtomicUsize::new(0));
    let pub_available = Arc::new(AtomicBool::new(true));
    (
        BarePublisher {
            buffer: buffer.clone(),
            size,
            wi: wi.clone(),
            sub_cnt: sub_cnt.clone(),
            pub_available: pub_available.clone(),
        },
        BareSubscriber {
            buffer,
            size,
            wi,
            ri: AtomicUsize::new(0),
            sub_cnt,
            pub_available,
        },
    )
}

impl<T: Send> BarePublisher<T> {
    /// Publishes values to the circular buffer at wi % size
    /// # Arguments
    /// * `object` - owned object to be published
    pub fn broadcast(&mut self, object: T) -> Result<(), SendError<T>> {
        if self.sub_cnt() == 0 {
            return Err(SendError(object));
        }
        self.buffer[self.wi() % self.size].store(Some(Arc::new(object)));
        self.wi_inc();
        Ok(())
    }
    #[inline(always)]
    fn wi(&self)->usize{
        self.wi.load(Ordering::Acquire)
    }
    #[inline(always)]
    fn wi_inc(&self){
        self.wi.fetch_add(1, Ordering::Release);
    }
    #[inline(always)]
    fn sub_cnt(&self)->usize{
        self.sub_cnt.load(Ordering::Relaxed)
    }
}
/// Drop trait is used to let subscribers know that publisher is no longer available.
impl<T: Send> Drop for BarePublisher<T> {
    fn drop(&mut self) {
        self.pub_available.store(false, Ordering::Relaxed);
    }
}

impl<T: Send> BareSubscriber<T> {
    /// Receives some atomic reference to an object if queue is not empty, or None if it is. Never
    /// Blocks
    #[inline(always)]
    fn ri(&self) -> usize {
        self.ri.load(Ordering::Relaxed)
    }

    #[inline(always)]
    fn wi(&self) -> usize {
        self.wi.load(Ordering::Acquire)
    }
    #[inline(always)]
    fn ri_store(&self, to_store: usize) {
        self.ri.store(to_store, Ordering::Relaxed);
    }
    #[inline(always)]
    fn ri_inc(&self){
        self.ri.fetch_add(1, Ordering::Relaxed);
    }
    fn pub_available(&self)->bool{
        self.pub_available.load(Ordering::Relaxed)
    }
    pub fn try_recv(&self) -> Result<Arc<T>, TryRecvError> {
        if self.ri() == self.wi() {
            if self.pub_available() {
                return Err(TryRecvError::Empty);
            } else {
                return Err(TryRecvError::Disconnected);
            }
        }

        loop {
            let val = self.buffer[self.ri() % self.size].load().unwrap();

            if self.wi() >= self.ri() + self.size {
                self.ri_store(self.wi() - self.size + 1);
            } else {
                self.ri_inc();
                return Ok(val);
            }
        }
    }
}

/// Clone trait is used to create another BareSubscriber object, subscribed to the same
/// Publisher the initial object was subscribed to.
impl<T: Send> Clone for BareSubscriber<T> {
    fn clone(&self) -> Self {
        self.sub_cnt.fetch_add(1, Ordering::Release);
        Self {
            buffer: self.buffer.clone(),
            wi: self.wi.clone(),
            ri: AtomicUsize::new(self.ri.load(Ordering::Relaxed)),
            size: self.size,
            sub_cnt: self.sub_cnt.clone(),
            pub_available: self.pub_available.clone(),
        }
    }
}

impl<T: Send> Drop for BareSubscriber<T> {
    fn drop(&mut self) {
        self.sub_cnt.fetch_sub(1, Ordering::Relaxed);
    }
}

impl<T: Send> Iterator for BareSubscriber<T> {
    type Item = Arc<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.try_recv().ok()
    }
}
/// Helper struct used by sync and async implementations to wake Tasks / Threads
#[derive(Debug)]
struct Waker<T> {
    /// Vector of Tasks / Threads to be woken up.
    pub sleepers: Vec<Arc<T>>,
    /// A mpsc Receiver used to receive Tasks / Threads to be registered.
    receiver: mpsc::Receiver<Arc<T>>,
}

/// Helper struct used by sync and async implementations to register Tasks / Threads to
/// be woken up.
#[derive(Debug)]
struct Sleeper<T> {
    /// Current Task / Thread to be woken up.
    pub sleeper: Arc<T>,
    /// mpsc Sender used to register Task / Thread.
    pub sender: mpsc::Sender<Arc<T>>,
}

impl<T> Waker<T> {
    /// Register all the Tasks / Threads sent for registration.
    pub fn register_receivers(&mut self) {
        for receiver in self.receiver.try_recv() {
            self.sleepers.push(receiver);
        }
    }
}

/// Function used to create a ( Waker, Sleeper ) tuple.
fn alarm<T>(current: T) -> (Waker<T>, Sleeper<T>) {
    let mut vec = Vec::new();
    let (sender, receiver) = mpsc::channel();
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
pub mod async;
