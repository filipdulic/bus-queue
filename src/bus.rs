use crate::channel::{bounded as raw_bounded, Receiver, SendError, Sender, TryRecvError};
use futures::future::Future;
use futures::{
    task::{self, Poll},
    Sink, Stream,
};
use piper::{Event, EventListener};
use std::pin::Pin;
use std::sync::Arc;

pub fn bounded<T>(size: usize) -> (Publisher<T>, Subscriber<T>) {
    let (sender, receiver) = raw_bounded(size);
    let event = Arc::new(Event::new());
    (
        Publisher {
            sender,
            event: event.clone(),
        },
        Subscriber {
            receiver,
            event,
            listener: None,
        },
    )
}

pub struct Publisher<T> {
    sender: Sender<T>,
    event: Arc<Event>,
}

impl<T> Sink<T> for Publisher<T> {
    type Error = SendError<T>;

    fn poll_ready(
        self: Pin<&mut Self>,
        _: &mut task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        self.sender.broadcast(item).and_then(|_| Ok(()))
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _: &mut task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.event.notify_all();
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.sender.close();
        self.poll_flush(cx)
    }
}

impl<T> PartialEq for Publisher<T> {
    fn eq(&self, other: &Publisher<T>) -> bool {
        self.sender == other.sender
    }
}

impl<T> Drop for Publisher<T> {
    fn drop(&mut self) {
        self.sender.close();
        self.event.notify_all();
    }
}

impl<T> Eq for Publisher<T> {}

pub struct Subscriber<T> {
    receiver: Receiver<T>,
    event: Arc<Event>,
    listener: Option<EventListener>,
}

impl<T> std::fmt::Debug for Subscriber<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscriber").finish()
    }
}

impl<T> Subscriber<T> {
    pub fn set_skip_items(self, skip_items: usize) {
        self.receiver.set_skip_items(skip_items);
    }
}

impl<T> Stream for Subscriber<T> {
    type Item = Arc<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            // If this stream is blocked on an event, first make sure it is unblocked.
            if let Some(listener) = self.listener.as_mut() {
                futures::ready!(Pin::new(listener).poll(cx));
                self.listener = None;
            }
            loop {
                // Attempt to receive a message.
                match self.receiver.try_recv() {
                    Ok(item) => {
                        // The stream is not blocked on an event - drop the listener.
                        self.listener = None;
                        return Poll::Ready(Some(item));
                    }
                    Err(TryRecvError::Disconnected) => {
                        // The stream is not blocked on an event - drop the listener.
                        self.listener = None;
                        return Poll::Ready(None);
                    }
                    Err(TryRecvError::Empty) => {}
                }
                // Listen for a send event.
                match self.listener.as_mut() {
                    None => {
                        // Store a listener and try sending the message again.
                        self.listener = Some(self.event.listen())
                    }
                    Some(_) => {
                        // Go back to the outer loop to poll the listener.
                        break;
                    }
                }
            }
        }
    }
}

impl<T> Clone for Subscriber<T> {
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver.clone(),
            event: self.event.clone(),
            listener: None,
        }
    }
}

impl<T: Send> PartialEq for Subscriber<T> {
    fn eq(&self, other: &Subscriber<T>) -> bool {
        self.receiver == other.receiver
    }
}

impl<T: Send> Eq for Subscriber<T> {}

#[cfg(test)]
mod test {
    use futures::future::FutureExt;
    use futures::pin_mut;
    use futures::task::Poll;
    use futures::SinkExt;
    use futures_test::task::noop_context;
    use futures_test::{assert_stream_done, assert_stream_next, assert_stream_pending};
    use std::sync::Arc;

    #[test]
    fn subscriber_is_in_pending_state_before_first_data_is_published() {
        let (_publisher, subscriber) = super::bounded::<usize>(1);
        pin_mut!(subscriber);

        // Assert that subscriber stream is pending before the publisher publishes.
        assert_stream_pending!(subscriber);
    }

    #[test]
    fn subscriber_receives_an_item_after_it_is_published() {
        let mut cx = noop_context();
        let (publisher, subscriber) = super::bounded::<usize>(1);
        pin_mut!(subscriber);
        pin_mut!(publisher);

        // Publish one item (1).
        assert_eq!(publisher.send(1).poll_unpin(&mut cx), Poll::Ready(Ok(())));

        // Assert that the subscriber can receive item (1).
        assert_stream_next!(subscriber, Arc::new(1));
    }

    #[test]
    fn subscriber_recieves_an_item_after_publisher_overflowed() {
        let mut cx = noop_context();
        let (publisher, subscriber) = super::bounded::<usize>(1);
        pin_mut!(subscriber);
        pin_mut!(publisher);

        // Publish item (1).
        assert_eq!(publisher.send(1).poll_unpin(&mut cx), Poll::Ready(Ok(())));

        // Assert that the publisher is not blocked even when overflowed
        // by publishing another item (2) while queue size is 1
        assert_eq!(publisher.send(2).poll_unpin(&mut cx), Poll::Ready(Ok(())));

        // Assert that the subscriber receives the second item (2),
        // since the first one (1) was dropped
        assert_stream_next!(subscriber, Arc::new(2));
    }
    #[test]
    fn subscriber_is_done_after_publisher_closes() {
        let mut cx = noop_context();
        let (publisher, subscriber) = super::bounded::<usize>(1);
        pin_mut!(subscriber);
        pin_mut!(publisher);

        // Close Publisher.
        assert_eq!(publisher.close().poll_unpin(&mut cx), Poll::Ready(Ok(())));

        // Assert that the subscriber is done..
        assert_stream_done!(subscriber);
    }

    #[test]
    fn subscriber_is_done_after_publisher_drop() {
        let (publisher, subscriber) = super::bounded::<usize>(1);
        pin_mut!(subscriber);

        // Drop Publisher
        drop(publisher);

        // Assert that the subscriber is done.
        assert_stream_done!(subscriber);
    }

    #[test]
    fn notify() {
        let (publisher, subscriber) = super::bounded::<usize>(1);
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
