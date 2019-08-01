use super::*;
use futures::prelude::*;
use futures::{Async, AsyncSink};

#[derive(Debug)]
pub struct Publisher<T: Send> {
    bare_publisher: BarePublisher<T>,
}
#[derive(Debug)]
pub struct Subscriber<T: Send> {
    bare_subscriber: BareSubscriber<T>,
}

pub fn channel<T: Send>(size: usize) -> (Publisher<T>, Subscriber<T>) {
    let (bare_publisher, bare_subscriber) = bare_channel(size);
    (Publisher { bare_publisher }, Subscriber { bare_subscriber })
}

impl<T: Send> GetSubCount for Publisher<T> {
    fn get_sub_count(&self) -> usize {
        self.bare_publisher.get_sub_count()
    }
}

impl<T: Send> Sink for Publisher<T> {
    type SinkItem = T;
    type SinkError = SendError<T>;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.bare_publisher
            .broadcast(item)
            .map(|_| AsyncSink::Ready)
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
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

impl<T: Send> PartialEq for Publisher<T> {
    fn eq(&self, other: &Publisher<T>) -> bool {
        self.bare_publisher == other.bare_publisher
    }
}

impl<T: Send> Eq for Publisher<T> {}

impl<T: Send> Stream for Subscriber<T> {
    type Item = Arc<T>;
    type Error = ();
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.bare_subscriber.try_recv() {
            Ok(arc_object) => Ok(Async::Ready(Some(arc_object))),
            Err(error) => match error {
                TryRecvError::Empty => {
                    futures::task::current().notify();
                    Ok(Async::NotReady)
                }
                TryRecvError::Disconnected => Ok(Async::Ready(None)),
            },
        }
    }
}

impl<T: Send> Clone for Subscriber<T> {
    fn clone(&self) -> Self {
        Self {
            bare_subscriber: self.bare_subscriber.clone(),
        }
    }
}

impl<T: Send> PartialEq for Subscriber<T> {
    fn eq(&self, other: &Subscriber<T>) -> bool {
        self.bare_subscriber == other.bare_subscriber
    }
}

impl<T: Send> Eq for Subscriber<T> {}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn async_channel() {
        let (tx, rx) = async_::channel(10);

        let rx2 = rx.clone();
        let rx3 = rx.clone();
        let rx4 = rx.clone();

        let pub_handle = std::thread::spawn(|| {
            let mut rt = Runtime::new().unwrap();
            let sent: Vec<_> = (1..15).collect();
            let publisher = futures::stream::iter_ok(sent)
                .forward(tx)
                .and_then(|(_, mut sink)| sink.close())
                .map_err(|_| ())
                .map(|_| ());

            rt.block_on(publisher)
        });
        pub_handle.join().unwrap().unwrap();

        let sub_handle = std::thread::spawn(move || {
            let mut rt = Runtime::new().unwrap();
            // Only the last 10 elements are received because the subscribers started receiving late
            let expected: Vec<_> = (5..15).collect();
            let received: Vec<_> = rt.block_on(rx.map(|x| *x).collect()).unwrap();
            assert_eq!(expected, received);

            let received: Vec<_> = rt.block_on(rx2.map(|x| *x).collect()).unwrap();
            assert_eq!(expected, received);

            let received: Vec<_> = rt.block_on(rx3.map(|x| *x).collect()).unwrap();
            assert_eq!(expected, received);

            let received: Vec<_> = rt.block_on(rx4.map(|x| *x).collect()).unwrap();
            assert_eq!(expected, received);
        });

        sub_handle.join().unwrap();
    }
}
