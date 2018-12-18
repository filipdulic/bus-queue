#[cfg(feature = "async")]
extern crate futures;

extern crate arc_swap;

use arc_swap::ArcSwapOption;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Provides an interface for subscribers
///
/// Every BusReader that can keep up with the push frequency should recv every pushed object.
/// BusReaders unable to keep up will miss object once the writer's index wi is larger then
/// reader's index ri + size
pub struct BusReader<T> {
    buffer: Arc<Vec<ArcSwapOption<T>>>,
    wi: Arc<AtomicUsize>,
    ri: usize,
    size: usize,
}

impl<T> BusReader<T> {
    /// Receives some atomic refrence to an object if queue is not empty, or None if it is
    pub fn recv(&mut self) -> Option<Arc<T>> {
        if self.ri == self.wi.load(Ordering::Relaxed) {
            return None;
        }
        loop {
            match self.buffer.get(self.ri % self.size).unwrap().load() {
                None => return None,
                Some(some) => {
                    if self.wi.load(Ordering::Relaxed) > self.ri + self.size {
                        self.ri = self.wi.load(Ordering::Relaxed) - self.size;
                    } else {
                        self.ri += 1;
                        return Some(some);
                    }
                }
            }
        }
    }
}

/// Provides an interface for the publisher
pub struct Bus<T> {
    // atp to an array of atps of option<arc<t>>
    buffer: Arc<Vec<ArcSwapOption<T>>>,
    wi: Arc<AtomicUsize>,
    size: usize,
}

impl<T> Bus<T> {
    /// Instantiates the Bus struct, creating the initial buffer filled with None.
    /// # Arguments
    /// * `size` - a usize size of the internal circular buffer
    pub fn new(size: usize) -> Self {
        let mut temp: Vec<ArcSwapOption<T>> = Vec::new();
        temp.resize(size, ArcSwapOption::new(None));

        Self {
            buffer: Arc::new(temp),
            wi: Arc::new(AtomicUsize::new(0)),
            size: size,
        }
    }
    /// Instantiates the BusReader struct connected the Bus circular buffer
    pub fn add_sub(&self) -> BusReader<T> {
        BusReader {
            buffer: self.buffer.clone(),
            wi: self.wi.clone(),
            ri: 0,
            size: self.size,
        }
    }
    /// Publishes values to the circular buffer at wi % size
    /// # Arguments
    /// * `object` - owned object to be published
    pub fn push(&mut self, object: T) {
        self.buffer[self.wi.load(Ordering::Relaxed) % self.size].store(Some(Arc::new(object)));
        self.wi.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(feature = "async")]
pub mod async {
    use super::{Arc};
    use super::arc_swap::ArcSwap;
    use super::{Bus, BusReader};
    use futures::prelude::*;
    use futures::task::{current, Task};
    use futures::{Async::NotReady, Async::Ready};
    use std::sync::mpsc::{channel,Receiver,Sender};

    pub struct AsyncBusReader<T> {
        reader: BusReader<T>,
        task_sender: Sender<Task>,
    }

    impl<T> Stream for AsyncBusReader<T> {
        type Item = Arc<T>;
        type Error = ();
        fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
            match self.reader.recv() {
                Some(arc_object) => Ok(Ready(Some(arc_object))),
                None => {
                    println!("got nothing");
                    self.task_sender.send(current());
                    Ok(NotReady)
                }
            }
        }
    }

    pub struct AsyncBus<T> {
        bus: Bus<T>,
        task_sender: Sender<Task>,
        task_receiver: Receiver<Task>
    }

    impl<T> AsyncBus<T> {
        pub fn new(size: usize) -> Self {
            let (tx,rx) = channel();
            Self {
                bus: Bus::new(size),
                task_receiver: rx,
                task_sender: tx,
            }
        }
        pub fn add_sub(&mut self) -> AsyncBusReader<T> {
            AsyncBusReader {
                reader: self.bus.add_sub(),
                task_sender: self.task_sender.clone(),
            }
        }
        pub fn push(&mut self, object: T) {
            self.bus.push(object);
            for task in self.task_receiver.try_recv() {
                task.notify();
            }
        }
    }
}
