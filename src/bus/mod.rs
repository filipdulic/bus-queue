mod publisher;
mod subscriber;

pub use publisher::Publisher;
pub use subscriber::Subscriber;

#[cfg(test)]
mod test {
    use crate::bounded as default_bounded;
    use futures::{pin_mut, task::Poll, FutureExt, SinkExt};
    use futures_test::task::noop_context;
    use futures_test::{assert_stream_done, assert_stream_next, assert_stream_pending};
    use std::sync::Arc;

    #[test]
    fn subscriber_is_in_pending_state_before_first_data_is_published() {
        let (_publisher, subscriber) = default_bounded::<usize>(1);
        pin_mut!(subscriber);

        // Assert that subscriber stream is pending before the publisher publishes.
        assert_stream_pending!(subscriber);
    }

    #[test]
    fn subscriber_receives_an_item_after_it_is_published() {
        let mut cx = noop_context();
        let (publisher, subscriber) = default_bounded::<usize>(1);
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
        let (publisher, subscriber) = default_bounded::<usize>(1);
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
        let (publisher, subscriber) = default_bounded::<usize>(1);
        pin_mut!(subscriber);
        pin_mut!(publisher);

        // Close Publisher.
        assert_eq!(publisher.close().poll_unpin(&mut cx), Poll::Ready(Ok(())));

        // Assert that the subscriber is done..
        assert_stream_done!(subscriber);
    }

    #[test]
    fn subscriber_is_done_after_publisher_drop() {
        let (publisher, subscriber) = default_bounded::<usize>(1);
        pin_mut!(subscriber);

        // Drop Publisher
        drop(publisher);

        // Assert that the subscriber is done.
        assert_stream_done!(subscriber);
    }

    #[test]
    fn notify() {
        let (publisher, subscriber) = default_bounded::<usize>(1);
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
    #[test]
    fn test_set_skip_items() {
        let (publisher, subscriber1) = default_bounded(3);
        let mut subscriber2 = subscriber1.clone();
        let mut subscriber3 = subscriber1.clone();
        let mut subscriber4 = subscriber1.clone();
        subscriber2.set_skip_items(1);
        subscriber3.set_skip_items(2);
        subscriber4.set_skip_items(3);

        pin_mut!(publisher);
        pin_mut!(subscriber1);
        pin_mut!(subscriber2);
        pin_mut!(subscriber3);
        pin_mut!(subscriber4);

        let mut cx = noop_context();
        for i in 0..6 {
            assert_eq!(publisher.send(i).poll_unpin(&mut cx), Poll::Ready(Ok(())));
        }
        assert_stream_next!(subscriber1, Arc::new(3));
        assert_stream_next!(subscriber2, Arc::new(4));
        assert_stream_next!(subscriber3, Arc::new(5));
        assert_stream_next!(subscriber4, Arc::new(5));
    }

    #[test]
    fn test_publisher_eq() {
        let (publisher1, _) = default_bounded::<i32>(1);
        let (publisher2, _) = default_bounded::<i32>(1);
        assert!(!publisher1.eq(&publisher2));
        assert!(publisher1.eq(&publisher1));
        assert!(publisher2.eq(&publisher2));
    }

    #[test]
    fn test_subscriber_eq() {
        let (_, subscriber1) = default_bounded::<i32>(1);
        let subscriber2 = subscriber1.clone();
        let (_, subscriber3) = default_bounded::<i32>(1);
        assert_eq!(subscriber1, subscriber2);
        assert_ne!(subscriber2, subscriber3);
        assert_ne!(subscriber1, subscriber3);
    }
}
