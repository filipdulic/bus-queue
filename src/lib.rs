//! # Lock-free Bounded Non-Blocking Pub-Sub Queue
//!
//!  This is a publish subscribe pattern queue, where the publisher is never blocked by
//!  slow subscribers. The side effect is that slow subscribers will miss messages. The intended
//!  use-case are high throughput streams where receiving the latest message is prioritized over
//!  receiving the entire stream. Market Data Feeds, Live Streams, etc....
//!
//!  The underlying data-structure is a vector of Arc(s) eliminating the use of copies.
//!
//! ## Features
//!  * Lock-Free Write/Read - Lock-Free for Publisher and Lock-Free for Subscribers.
//!  * Bounded - Constant size of memory used, max is **sizeof(MsgObject)*(queue_size + sub_cnt + 1)**.
//!    This is an edge-case where each subscriber is holding a ref to an object while the publisher
//!    has published a full length of queue in the mean time.
//!  * Non-Blocking - The queue never blocks the publisher, slow subscribers miss data proportinal to
//!    their speed.
//!  * Pub-Sub - Every Subscriber that can keep up with the Publisher will recieve all the data the
//!    Publisher publishes.
//!  * **channel** - a raw Pub/Sub channel implementation without the thread synchronisation and futures logic.
//!  * **bus** - an async Pub/Sub queue with **futures::sink::Sink** and **futures::stream::Stream** traits.
//!
//! **bus::Publisher**, and **channel::Sender** are used to broadcast data to **bus::Subscriber**, and
//! **channel::Receiver** pools. Subscribers are clone-able such that many threads, or futures, can receive
//! data simultaneously. The only limitation is that Subscribers have to keep up with the frequency of the
//! Publisher. If a Subscriber is slow it will drop data.
//!
//! ## Disconnection
//!
//! The broadcast and receive operations on channels will all return a **Result**
//! indicating whether the operation succeeded or not. An unsuccessful operation
//! is normally indicative of the other half of a channel having "hung up" by
//! being dropped in its corresponding thread.
//!
//! Once half of a channel has been deallocated, most operations can no longer
//! continue to make progress, so **Err** will be returned. Many applications
//! will continue to **unwrap** the results returned from this module,
//! instigating a propagation of failure among threads if one unexpectedly dies.
//!
//!
//! # Examples
//!
//! ## Simple raw usage
//!
//! ```rust
//! extern crate bus_queue;
//! use bus_queue::flavors::arc_swap::bounded;
//!
//! let (tx, rx) = bounded(10);
//! (1..15).for_each(|x| tx.broadcast(x).unwrap());
//!
//! let received: Vec<i32> = rx.map(|x| *x).collect();
//! // Test that only the last 10 elements are in the received list.
//! let expected: Vec<i32> = (5..15).collect();
//!
//! assert_eq!(expected, received);
//! ```
//!
//! ## Simple async usage
//!
//! ```rust
//! use bus_queue::flavors::arc_swap::async_bounded;
//! use futures::executor::block_on;
//! use futures::stream;
//! use futures::StreamExt;
//!
//! let (publisher, subscriber1) = async_bounded(10);
//! let subscriber2 = subscriber1.clone();
//!
//! block_on(async move {
//!     stream::iter(1..15)
//!         .map(|i| Ok(i))
//!         .forward(publisher)
//!         .await
//!         .unwrap();
//! });
//!
//! let received1: Vec<u32> = block_on(async { subscriber1.map(|x| *x).collect().await });
//! let received2: Vec<u32> = block_on(async { subscriber2.map(|x| *x).collect().await });
//! // Test that only the last 10 elements are in the received list.
//! let expected = (5..15).collect::<Vec<u32>>();
//! assert_eq!(received1, expected);
//! assert_eq!(received2, expected);
//! ```

mod async_publisher;
mod async_subscriber;
mod atomic_counter;
pub mod flavors;
mod publisher;
mod ring_buffer;
mod subscriber;
mod swap_slot;

pub use crate::async_publisher::AsyncPublisher;
pub use crate::async_subscriber::AsyncSubscriber;
pub use crate::publisher::Publisher;
pub use crate::subscriber::Subscriber;
pub use ring_buffer::RingBuffer;
pub use swap_slot::SwapSlot;

#[cfg(feature = "atomic-arc")]
mod atomic;

pub use atomic_counter::AtomicCounter;

/// Function used to create and initialise a (Sender, Receiver) tuple.
pub fn bounded<T, S: SwapSlot<T>>(
    size: usize,
) -> (publisher::Publisher<T, S>, subscriber::Subscriber<T, S>) {
    use std::sync::Arc;
    let arc_channel = Arc::new(RingBuffer::new(size));
    (
        publisher::Publisher::from(arc_channel.clone()),
        subscriber::Subscriber::from(arc_channel),
    )
}

pub fn async_bounded<T, S: SwapSlot<T>>(
    size: usize,
) -> (
    async_publisher::AsyncPublisher<T, S>,
    async_subscriber::AsyncSubscriber<T, S>,
) {
    use event_listener::Event;
    use std::sync::Arc;
    let (publisher, subscriber) = bounded(size);
    let event = Arc::new(Event::new());
    (
        async_publisher::AsyncPublisher::from((publisher, event.clone())),
        async_subscriber::AsyncSubscriber::from((subscriber, event)),
    )
}
