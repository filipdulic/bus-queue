use super::sync;
use futures::prelude::*;
use futures::{task::AtomicTask, Async, AsyncSink};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::{mpsc, mpsc::Receiver, mpsc::Sender, Arc};

pub struct Publisher<T: Send> {
    publisher: sync::Publisher<T>,
    tasks: Vec<Arc<AtomicTask>>,
    sink_closed: Arc<AtomicBool>,
    task_receiver: Receiver<Arc<AtomicTask>>,
}

pub struct Subscriber<T: Send> {
    subscriber: sync::Subscriber<T>,
    task: Arc<AtomicTask>,
    sink_closed: Arc<AtomicBool>,
    task_sender: Sender<Arc<AtomicTask>>,
}

pub fn channel<T: Send>(size: usize) -> (Publisher<T>, Subscriber<T>) {
    let (publisher, subscriber) = sync::channel(size);
    let (task_sender, task_receiver) = mpsc::channel();
    let closed = Arc::new(AtomicBool::new(false));
    let arc = Arc::new(AtomicTask::new());
    task_sender.send(arc.clone()).unwrap();
    (
        Publisher {
            publisher,
            tasks: Vec::new(),
            task_receiver,
            sink_closed: closed.clone(),
        },
        Subscriber {
            subscriber,
            task: arc,
            sink_closed: closed.clone(),
            task_sender,
        },
    )
}

impl<T: Send> Sink for Publisher<T> {
    type SinkItem = T;
    type SinkError = ();

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        for task in self.task_receiver.try_recv().into_iter() {
            self.tasks.push(task);
        }
        self.publisher.broadcast(item);
        Ok(AsyncSink::Ready)
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        for t in &self.tasks {
            t.notify();
        }
        Ok(Async::Ready(()))
    }
    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.sink_closed.store(true, Relaxed);
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
        match self.subscriber.recv() {
            Some(arc_object) => Ok(Async::Ready(Some(arc_object))),
            None => {
                if self.sink_closed.load(Relaxed) == true {
                    // Stream closed no further data will be available ever
                    Ok(Async::Ready(None))
                } else {
                    self.task.register();
                    Ok(Async::NotReady)
                }
            }
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
            sink_closed: self.sink_closed.clone(),
        }
    }
}
