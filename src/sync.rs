use super::*;
use arc_swap::ArcSwapOption;
#[allow(unused_imports)]
use std::iter::{IntoIterator, Iterator};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
#[allow(unused_imports)]
use std::time::{Duration, Instant};

/// Provides an interface for the publisher
pub struct Publisher<T: Send> {
    // atp to an array of atps of option<arc<t>>
    buffer: Arc<Vec<ArcSwapOption<T>>>,
    wi: Arc<AtomicUsize>,
    size: usize,
    sub_cnt: Arc<AtomicUsize>,
    pub_available: Arc<AtomicBool>,
    cv: Arc<Condvar>,
}

/// Provides an interface for subscribers
///
/// Every BusReader that can keep up with the push frequency should recv every pushed object.
/// BusReaders unable to keep up will miss object once the writer's index wi is larger then
/// reader's index ri + size
#[derive(Debug)]
pub struct Subscriber<T: Send> {
    buffer: Arc<Vec<ArcSwapOption<T>>>,
    wi: Arc<AtomicUsize>,
    ri: AtomicUsize,
    size: usize,
    sub_cnt: Arc<AtomicUsize>,
    pub_available: Arc<AtomicBool>,
    cv: Arc<Condvar>,
    mtx: Arc<Mutex<bool>>,
}

pub fn channel<T: Send>(size: usize) -> (Publisher<T>, Subscriber<T>) {
    let mut buffer = Vec::new();
    buffer.resize(size, ArcSwapOption::new(None));
    let buffer = Arc::new(buffer);
    let sub_cnt = Arc::new(AtomicUsize::new(1));
    let wi = Arc::new(AtomicUsize::new(0));
    let pub_available = Arc::new(AtomicBool::new(true));
    let cv = Arc::new(Condvar::new());
    let mtx = Arc::new(Mutex::new(false));
    (
        Publisher {
            buffer: buffer.clone(),
            size,
            wi: wi.clone(),
            sub_cnt: sub_cnt.clone(),
            pub_available: pub_available.clone(),
            cv: cv.clone(),
        },
        Subscriber {
            buffer: buffer.clone(),
            size,
            wi: wi.clone(),
            ri: AtomicUsize::new(0),
            sub_cnt: sub_cnt.clone(),
            pub_available: pub_available.clone(),
            cv: cv.clone(),
            mtx: mtx.clone(),
        },
    )
}

impl<T: Send> Publisher<T> {
    /// Publishes values to the circular buffer at wi % size
    /// # Arguments
    /// * `object` - owned object to be published
    pub fn broadcast(&mut self, object: T) -> Result<(), SendError<T>> {
        if self.sub_cnt.load(Ordering::Relaxed) == 0 {
            return Err(SendError(object));
        }
        self.buffer[self.wi.load(Ordering::Relaxed) % self.size].store(Some(Arc::new(object)));
        self.wi.fetch_add(1, Ordering::Relaxed);
        self.cv.notify_all();
        Ok(())
    }
}

impl<T: Send> Drop for Publisher<T> {
    fn drop(&mut self) {
        self.pub_available.store(false, Ordering::Relaxed);
    }
}

impl<T: Send> Subscriber<T> {
    /// Receives some atomic refrence to an object if queue is not empty, or None if it is
    pub fn try_recv(&self) -> Result<Arc<T>, TryRecvError> {
        if self.pub_available.load(Ordering::Relaxed) == false {
            return Err(TryRecvError::Disconnected);
        }
        if self.ri.load(Ordering::Relaxed) == self.wi.load(Ordering::Relaxed) {
            return Err(TryRecvError::Empty);
        }
        loop {
            match self.buffer[self.ri.load(Ordering::Relaxed) % self.size].load() {
                Some(some) => if self.wi.load(Ordering::Relaxed)
                    > self.ri.load(Ordering::Relaxed) + self.size
                {
                    self.ri.store(
                        self.wi.load(Ordering::Relaxed) - self.size,
                        Ordering::Relaxed,
                    );
                } else {
                    self.ri.fetch_add(1, Ordering::Relaxed);
                    return Ok(some);
                },
                None => unreachable!(),
            }
        }
    }

    //    pub fn recv(&mut self) -> Result<Arc<T>, RecvError> {}
    //    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {}
    //    pub fn recv_deadline(&self, deadline: Instant) -> Result<T, RecvTimeoutError> {}
}

impl<T: Send> Clone for Subscriber<T> {
    fn clone(&self) -> Self {
        self.sub_cnt.fetch_add(1, Ordering::Relaxed);
        Self {
            buffer: self.buffer.clone(),
            wi: self.wi.clone(),
            ri: AtomicUsize::new(self.ri.load(Ordering::Relaxed)),
            size: self.size,
            sub_cnt: self.sub_cnt.clone(),
            pub_available: self.pub_available.clone(),
            cv: self.cv.clone(),
            mtx: self.mtx.clone(),
        }
    }
}

impl<T: Send> Drop for Subscriber<T> {
    fn drop(&mut self) {
        self.sub_cnt.fetch_sub(1, Ordering::Relaxed);
    }
}

//impl<'a, T> Iterator for &'a Subscriber<T> {}
//impl<T> Iterator for Subscriber<T>{}
//impl<'a, T> IntoIterator for &'a Subscriber<T> {}
//impl<T> IntoIterator for Subscriber<T>{}
