extern crate arc_swap;
use arc_swap::{ArcSwap, ArcSwapOption};
use std::sync::mpsc::{RecvError, RecvTimeoutError, SendError, TryRecvError};
use std::sync::{atomic::AtomicBool, atomic::AtomicUsize, atomic::Ordering, mpsc, Arc};

pub struct BarePublisher<T: Send> {
    buffer: Arc<Vec<ArcSwapOption<T>>>,
    wi: Arc<AtomicUsize>,
    size: usize,
    sub_cnt: Arc<AtomicUsize>,
    pub_available: Arc<AtomicBool>,
}

pub struct BareSubscriber<T: Send> {
    buffer: Arc<Vec<ArcSwapOption<T>>>,
    wi: Arc<AtomicUsize>,
    ri: AtomicUsize,
    size: usize,
    sub_cnt: Arc<AtomicUsize>,
    pub_available: Arc<AtomicBool>,
}

pub fn bare_channel<T: Send>(size: usize) -> (BarePublisher<T>, BareSubscriber<T>) {
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
            buffer: buffer.clone(),
            size,
            wi: wi.clone(),
            ri: AtomicUsize::new(0),
            sub_cnt: sub_cnt.clone(),
            pub_available: pub_available.clone(),
        },
    )
}

impl<T: Send> BarePublisher<T> {
    /// Publishes values to the circular buffer at wi % size
    /// # Arguments
    /// * `object` - owned object to be published
    pub fn broadcast(&mut self, object: T) -> Result<(), SendError<T>> {
        if self.sub_cnt.load(Ordering::Relaxed) == 0 {
            return Err(SendError(object));
        }
        self.buffer[self.wi.load(Ordering::Relaxed) % self.size].store(Some(Arc::new(object)));
        self.wi.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

impl<T: Send> Drop for BarePublisher<T> {
    fn drop(&mut self) {
        self.pub_available.store(false, Ordering::Relaxed);
    }
}

impl<T: Send> BareSubscriber<T> {
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
}

impl<T: Send> Clone for BareSubscriber<T> {
    fn clone(&self) -> Self {
        self.sub_cnt.fetch_add(1, Ordering::Relaxed);
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

pub struct Waker<T> {
    pub sleepers: Vec<Arc<ArcSwap<T>>>,
    receiver: mpsc::Receiver<Arc<ArcSwap<T>>>,
}

pub struct Sleeper<T> {
    pub sleeper: Arc<ArcSwap<T>>,
    pub sender: mpsc::Sender<Arc<ArcSwap<T>>>,
}

impl<T> Waker<T> {
    pub fn register_receivers(&mut self) {
        for receiver in self.receiver.try_recv() {
            self.sleepers.push(receiver);
        }
    }
}

impl<T> Sleeper<T> {
    pub fn send(&self, current: Arc<ArcSwap<T>>) {
        self.sender.send(current).unwrap();
    }
    pub fn register(&self, t: T) {
        self.sleeper.store(Arc::new(t));
    }
}

fn alarm<T>(current: T) -> (Waker<T>, Sleeper<T>) {
    let mut vec = Vec::new();
    let (sender, receiver) = mpsc::channel();
    let arc_t = Arc::new(ArcSwap::new(Arc::new(current)));
    vec.push(arc_t.clone());
    (
        Waker {
            sleepers: vec,
            receiver,
        },
        Sleeper {
            sleeper: arc_t.clone(),
            sender,
        },
    )
}

pub mod sync;
#[cfg(feature = "async")]
extern crate futures;
#[cfg(feature = "async")]
pub mod async;
