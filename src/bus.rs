use crate::channel::{bounded as raw_bounded, Receiver, SendError, Sender, TryRecvError};
use crossbeam_channel as mpsc;
use futures::task::AtomicWaker;
use futures::{
    task::{self, Poll},
    Sink, Stream,
};
use std::pin::Pin;
use std::sync::Arc;

pub fn bounded<T>(size: usize) -> (Publisher<T>, Subscriber<T>) {
    let (sender, receiver) = raw_bounded(size);
    let (waker, sleeper) = alarm();
    (
        Publisher { sender, waker },
        Subscriber { receiver, sleeper },
    )
}

#[derive(Debug)]
pub struct Publisher<T> {
    sender: Sender<T>,
    waker: Waker,
}

impl<T> Sink<T> for Publisher<T> {
    type Error = SendError<T>;

    fn poll_ready(
        self: Pin<&mut Self>,
        _: &mut task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        self.waker.collect_new_wakers();
        self.sender.broadcast(item).and_then(|_| {
            self.waker.wake_all();
            Ok(())
        })
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _: &mut task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: Pin<&mut Self>,
        _: &mut task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl<T> PartialEq for Publisher<T> {
    fn eq(&self, other: &Publisher<T>) -> bool {
        self.sender == other.sender
    }
}

impl<T> Eq for Publisher<T> {}

pub struct Subscriber<T> {
    receiver: Receiver<T>,
    sleeper: Sleeper,
}

impl<T> Subscriber<T> {
    pub fn set_skip_items(self, skip_items: usize) {
        self.receiver.set_skip_items(skip_items);
    }
}

impl<T> Stream for Subscriber<T> {
    type Item = Arc<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Option<Self::Item>> {
        self.sleeper.register(cx.waker());
        match self.receiver.try_recv() {
            Ok(item) => Poll::Ready(Some(item)),
            Err(error) => match error {
                TryRecvError::Empty => Poll::Pending,
                TryRecvError::Disconnected => Poll::Ready(None),
            },
        }
    }
}

impl<T> Clone for Subscriber<T> {
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver.clone(),
            sleeper: self.sleeper.clone(),
        }
    }
}

impl<T: Send> PartialEq for Subscriber<T> {
    fn eq(&self, other: &Subscriber<T>) -> bool {
        self.receiver == other.receiver
    }
}

impl<T: Send> Eq for Subscriber<T> {}

// Helper struct used by sync and async implementations to wake Tasks / Threads
#[derive(Debug)]
pub struct Waker {
    /// Vector of Wakers to use to wake up subscribers.
    wakers: Vec<Arc<AtomicWaker>>,
    /// A mpsc Receiver used to receive Wakers
    receiver: mpsc::Receiver<Arc<AtomicWaker>>,
}

impl Waker {
    fn wake_all(&self) {
        for waker in &self.wakers {
            waker.wake();
        }
    }

    /// Receive any new Wakers and add them to the wakers Vec. These will be used to wake up the
    /// subscribers when a message is published
    fn collect_new_wakers(&mut self) {
        while let Ok(receiver) = self.receiver.try_recv() {
            self.wakers.push(receiver);
        }
    }
}

/// Helper struct used by sync and async implementations to register Tasks / Threads to
/// be woken up.
#[derive(Debug)]
pub struct Sleeper {
    /// Current Waker to be woken up
    waker: Arc<AtomicWaker>,
    /// mpsc Sender used to send Wakers to the Publisher
    sender: mpsc::Sender<Arc<AtomicWaker>>,
}

impl Sleeper {
    fn register(&self, waker: &task::Waker) {
        self.waker.register(waker);
    }
}

impl Clone for Sleeper {
    fn clone(&self) -> Self {
        let waker = Arc::new(AtomicWaker::new());
        // Send the new waker to the publisher.
        // If this fails (Receiver disconnected), presumably the Publisher
        // has dropped and when this is polled for the first time, the
        // Stream will end.
        let _ = self.sender.send(Arc::clone(&waker));
        Self {
            waker,
            sender: self.sender.clone(),
        }
    }
}

/// Function used to create a ( Waker, Sleeper ) tuple.
pub fn alarm() -> (Waker, Sleeper) {
    let (sender, receiver) = mpsc::unbounded();
    let waker = Arc::new(AtomicWaker::new());
    let wakers = vec![Arc::clone(&waker)];
    (Waker { wakers, receiver }, Sleeper { waker, sender })
}

#[cfg(test)]
mod test {
    use futures_test::{
        assert_stream_pending, assert_stream_next, assert_stream_done,
    };
    use futures_test::task::noop_context;
    use futures::pin_mut;
    use futures::SinkExt;
    use futures::task::Poll;
    use std::sync::Arc;
    use futures::future::FutureExt;

    #[test]
    fn subscriber_is_in_pending_state_before_first_data_is_published()
    {
        let (_publisher,  subscriber) = super::bounded::<usize>(1);
        pin_mut!(subscriber);

        // Assert that subscriber stream is pending before the publisher publishes.
        assert_stream_pending!(subscriber);
    }

    #[test]
    fn subscriber_receives_an_item_after_it_is_published()
    {
        let mut cx = noop_context();
        let (publisher,  subscriber) = super::bounded::<usize>(1);
        pin_mut!(subscriber);
        pin_mut!(publisher);

        // Publish one item (1).
        assert_eq!(publisher.send(1).poll_unpin(&mut cx), Poll::Ready(Ok(())));

        // Assert that the subscriber can receive item (1).
        assert_stream_next!(subscriber, Arc::new(1));
    }
    #[test]
    fn subscriber_recieves_an_item_after_publisher_overflowed()
    {
        let mut cx = noop_context();
        let (publisher,  subscriber) = super::bounded::<usize>(1);
        pin_mut!(subscriber);
        pin_mut!(publisher);

        // Publish 1 item (1).
        assert_eq!(publisher.send(1).poll_unpin(&mut cx), Poll::Ready(Ok(())));

        // Assert that the publisher is not blocked even when overflowed
        // by publishing another item (2) while queue size is 1
        assert_eq!(publisher.send(2).poll_unpin(&mut cx), Poll::Ready(Ok(())));

        // Assert that the subscriber receives the second item (2),
        // since the first one (1) was dropped
        assert_stream_next!(subscriber, Arc::new(2));
    }
    #[test]
    fn subscriber_is_done_after_publisher_closes()
    {
        let mut cx = noop_context();
        let (publisher,  subscriber) = super::bounded::<usize>(1);
        pin_mut!(subscriber);
        pin_mut!(publisher);

        // Close Publisher.
        assert_eq!(publisher.close().poll_unpin(&mut cx), Poll::Ready(Ok(())));

        // Assert that the subscriber can receive item (1).
        assert_stream_done!(subscriber);
    }

    #[test]
    fn notify()
    {
        let (publisher,  subscriber) = super::bounded::<usize>(1);
        pin_mut!(subscriber);
        pin_mut!(publisher);

        // Assert that subscriber stream is pending before the publisher publishes.
        assert_stream_pending!(subscriber);

        // Publish one item (1).
        let mut cx = noop_context();
        assert_eq!(publisher.send(1).poll_unpin(&mut cx), Poll::Ready(Ok(())));

        // Assert that the subscriber can receive item (1).
        assert_stream_next!(subscriber, Arc::new(1));

        // Publish one more item  (2).
        assert_eq!(publisher.send(2).poll_unpin(&mut cx), Poll::Ready(Ok(())));

        // Assert that the subscriber can receive item (2).
        assert_stream_next!(subscriber, Arc::new(2));

        // Assert that the subscirber is pending of another item to be published.
        assert_stream_pending!(subscriber);

        // Close publisher.
        assert_eq!(publisher.close().poll_unpin(&mut cx), Poll::Ready(Ok(())));

        // Assert that subscriber is done.
        assert_stream_done!(subscriber);
    }
}
