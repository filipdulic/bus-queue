use super::*;
use futures::prelude::*;
use futures::{task::AtomicTask, Async, AsyncSink};
use std::sync::{mpsc, mpsc::Receiver, mpsc::Sender, Arc};

pub struct Publisher<T: Send> {
    publisher: sync::Publisher<T>,
    tasks: Vec<Arc<AtomicTask>>,
    task_receiver: Receiver<Arc<AtomicTask>>,
}

pub struct Subscriber<T: Send> {
    subscriber: sync::Subscriber<T>,
    task: Arc<AtomicTask>,
    task_sender: Sender<Arc<AtomicTask>>,
}

pub fn channel<T: Send>(size: usize) -> (Publisher<T>, Subscriber<T>) {
    let (publisher, subscriber) = sync::channel(size);
    let (task_sender, task_receiver) = mpsc::channel();
    let arc = Arc::new(AtomicTask::new());
    task_sender.send(arc.clone()).unwrap();
    (
        Publisher {
            publisher,
            tasks: Vec::new(),
            task_receiver,
        },
        Subscriber {
            subscriber,
            task: arc,
            task_sender,
        },
    )
}

impl<T: Send> Sink for Publisher<T> {
    type SinkItem = T;
    type SinkError = SendError<T>;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        for task in self.task_receiver.try_recv().into_iter() {
            self.tasks.push(task);
        }
        match self.publisher.broadcast(item) {
            Ok(_) => Ok(AsyncSink::Ready),
            Err(e) => Err(e),
        }
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        for t in &self.tasks {
            t.notify();
        }
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
        match self.subscriber.try_recv() {
            Ok(arc_object) => Ok(Async::Ready(Some(arc_object))),
            Err(error) => match error {
                TryRecvError::Empty => {
                    self.task.register();
                    Ok(Async::NotReady)
                }
                TryRecvError::Disconnected => Ok(Async::Ready(None)),
            },
        }
    }
}

impl<T: Send> Clone for Subscriber<T> {
    fn clone(&self) -> Self {
        let arc = Arc::new(AtomicTask::new());
        self.task_sender.send(arc.clone()).unwrap();
        Self {
            subscriber: self.subscriber.clone(),
            task: arc.clone(),
            task_sender: self.task_sender.clone(),
        }
    }
}
