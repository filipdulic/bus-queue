use super::*;
use std::iter::Iterator;
use std::thread;
use std::time::{Duration, Instant};

/// Provides an interface for the publisher
#[derive(Debug)]
pub struct Publisher<T: Send> {
    bare_publisher: BarePublisher<T>,
    waker: Waker<ArcSwap<thread::Thread>>,
}

/// Provides an interface for subscribers
///
/// Every BusReader that can keep up with the push frequency should recv every pushed object.
/// BusReaders unable to keep up will miss object once the writer's index wi is larger then
/// reader's index ri + size
#[derive(Debug)]
pub struct Subscriber<T: Send> {
    bare_subscriber: BareSubscriber<T>,
    sleeper: Sleeper<ArcSwap<thread::Thread>>,
}

pub fn channel<T: Send>(size: usize) -> (Publisher<T>, Subscriber<T>) {
    let (bare_publisher, bare_subscriber) = bare_channel(size);
    let (waker, sleeper) = alarm(ArcSwap::new(Arc::new(thread::current())));
    (
        Publisher {
            bare_publisher,
            waker,
        },
        Subscriber {
            bare_subscriber,
            sleeper,
        },
    )
}

impl<T: Send> Publisher<T> {
    /// Publishes values to the circular buffer at wi % size
    /// # Arguments
    /// * `object` - owned object to be published
    pub fn broadcast(&mut self, object: T) -> Result<(), SendError<T>> {
        self.bare_publisher.broadcast(object)?;
        self.waker.register_receivers();
        self.wake_all();
        Ok(())
    }
    pub fn wake_all(&self) {
        for sleeper in self.waker.sleepers.iter() {
            sleeper.load().unpark();
        }
    }
}

impl<T: Send> GetSubCount for Publisher<T> {
    fn get_sub_count(&self) -> usize {
        self.bare_publisher.get_sub_count()
    }
}

impl<T: Send> Drop for Publisher<T> {
    fn drop(&mut self) {
        self.wake_all();
    }
}

impl<T: Send> PartialEq for Publisher<T> {
    fn eq(&self, other: &Publisher<T>) -> bool {
        self.bare_publisher == other.bare_publisher
    }
}

impl<T: Send> Eq for Publisher<T> {}

impl<T: Send> Subscriber<T> {
    pub fn try_recv(&self) -> Result<Arc<T>, TryRecvError> {
        self.bare_subscriber.try_recv()
    }
    pub fn recv(&self) -> Result<Arc<T>, RecvError> {
        loop {
            let result = self.bare_subscriber.try_recv();
            if let Ok(object) = result {
                return Ok(object);
            }
            if let Err(e) = result {
                if let TryRecvError::Disconnected = e {
                    return Err(RecvError);
                }
            }
            self.sleeper.sleeper.store(Arc::new(thread::current()));
            thread::park();
        }
    }
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Arc<T>, RecvTimeoutError> {
        loop {
            let result = self.bare_subscriber.try_recv();
            if let Ok(object) = result {
                return Ok(object);
            }
            if let Err(e) = result {
                if let TryRecvError::Disconnected = e {
                    return Err(RecvTimeoutError::Disconnected);
                }
            }
            self.sleeper.sleeper.store(Arc::new(thread::current()));
            let parking = Instant::now();
            thread::park_timeout(timeout);
            let unparked = Instant::now();
            if unparked.duration_since(parking) >= timeout {
                return Err(RecvTimeoutError::Timeout);
            }
        }
    }
}

impl<T: Send> Clone for Subscriber<T> {
    fn clone(&self) -> Self {
        let arc_t = Arc::new(ArcSwap::new(Arc::new(thread::current())));
        self.sleeper.sender.send(arc_t.clone()).unwrap();
        Self {
            bare_subscriber: self.bare_subscriber.clone(),
            sleeper: Sleeper {
                sender: self.sleeper.sender.clone(),
                sleeper: arc_t.clone(),
            },
        }
    }
}

impl<'a, T: Send> Iterator for &'a Subscriber<T> {
    type Item = Arc<T>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.recv() {
            Ok(item) => Some(item),
            Err(_) => None,
        }
    }
}
impl<T: Send> Iterator for Subscriber<T> {
    type Item = Arc<T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.recv().ok()
    }
}

impl<T: Send> PartialEq for Subscriber<T> {
    fn eq(&self, other: &Subscriber<T>) -> bool {
        self.bare_subscriber == other.bare_subscriber
    }
}

impl<T: Send> Eq for Subscriber<T> {}
