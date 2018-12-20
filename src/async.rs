use super::*;
use futures::prelude::*;
use futures::{task::AtomicTask, Async, AsyncSink};

pub struct Publisher<T: Send> {
    bare_publisher: BarePublisher<T>,
    waker: Waker<AtomicTask>,
}

pub struct Subscriber<T: Send> {
    bare_subscriber: BareSubscriber<T>,
    sleeper: Sleeper<AtomicTask>,
}

pub fn channel<T: Send>(size: usize) -> (Publisher<T>, Subscriber<T>) {
    let (bare_publisher, bare_subscriber) = bare_channel(size);
    let (waker, sleeper) = alarm(AtomicTask::new());
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
    fn wake_all(&self) {
        for sleeper in &self.waker.sleepers {
            sleeper.notify();
        }
    }
}

impl<T: Send> Sink for Publisher<T> {
    type SinkItem = T;
    type SinkError = SendError<T>;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.waker.register_receivers();
        match self.bare_publisher.broadcast(item) {
            Ok(_) => Ok(AsyncSink::Ready),
            Err(e) => Err(e),
        }
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.wake_all();
        Ok(Async::Ready(()))
    }
    fn close(&mut self) -> Poll<(), Self::SinkError> {
        Ok(Async::Ready(()))
    }
}

impl<T: Send> Drop for Publisher<T> {
    fn drop(&mut self) {
        self.close().unwrap();
        self.poll_complete().unwrap();
    }
}

impl<T: Send> Stream for Subscriber<T> {
    type Item = Arc<T>;
    type Error = ();
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.bare_subscriber.try_recv() {
            Ok(arc_object) => Ok(Async::Ready(Some(arc_object))),
            Err(error) => match error {
                TryRecvError::Empty => {
                    self.sleeper.sleeper.register();
                    Ok(Async::NotReady)
                }
                TryRecvError::Disconnected => Ok(Async::Ready(None)),
            },
        }
    }
}

impl<T: Send> Clone for Subscriber<T> {
    fn clone(&self) -> Self {
        let arc_t = Arc::new(AtomicTask::new());
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
