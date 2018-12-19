use arc_swap::ArcSwapOption;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;


/// Provides an interface for the publisher
pub struct Publisher<T> {
    // atp to an array of atps of option<arc<t>>
    buffer: Arc<Vec<ArcSwapOption<T>>>,
    wi: Arc<AtomicUsize>,
    size: usize,
}

/// Provides an interface for subscribers
///
/// Every BusReader that can keep up with the push frequency should recv every pushed object.
/// BusReaders unable to keep up will miss object once the writer's index wi is larger then
/// reader's index ri + size
pub struct Subscriber<T> {
    buffer: Arc<Vec<ArcSwapOption<T>>>,
    wi: Arc<AtomicUsize>,
    ri: usize,
    size: usize,
}

pub fn channel<T: Send>(size: usize) -> (Publisher<T>, Subscriber<T>) {
    let mut buffer = Vec::new();
    buffer.resize(size,ArcSwapOption::new(None));
    let buffer = Arc::new(buffer);

    let wi = Arc::new(AtomicUsize::new(0));
    (Publisher {
        buffer: buffer.clone(),
        size,
        wi: wi.clone(),
    }, Subscriber {
        buffer: buffer.clone(),
        size,
        wi: wi.clone(),
        ri: 0,
    })
}

impl<T> Publisher<T> {
    /// Publishes values to the circular buffer at wi % size
    /// # Arguments
    /// * `object` - owned object to be published
    pub fn broadcast(&mut self, object: T) {
        self.buffer[self.wi.load(Ordering::Relaxed) % self.size].store(Some(Arc::new(object)));
        self.wi.fetch_add(1, Ordering::Relaxed);
    }
}

impl<T> Subscriber<T> {
    /// Receives some atomic refrence to an object if queue is not empty, or None if it is
    pub fn recv(&mut self) -> Option<Arc<T>> {
        if self.ri == self.wi.load(Ordering::Relaxed) {
            return None;
        }
        loop {
            match self.buffer[self.ri % self.size].load() {
                Some(some) => if self.wi.load(Ordering::Relaxed) > self.ri + self.size {
                    self.ri = self.wi.load(Ordering::Relaxed) - self.size;
                } else {
                    self.ri += 1;
                    return Some(some);
                },
                None => unreachable!(),
            }
        }
    }
}

impl<T> Clone for Subscriber<T> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
            wi: self.wi.clone(),
            ri: self.ri,
            size: self.size,
        }
    }
}
