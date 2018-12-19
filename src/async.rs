use super::sync;
use futures::prelude::*;
use futures::{task::AtomicTask, Async, AsyncSink};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::{mpsc, mpsc::Receiver, mpsc::Sender, Arc};

pub struct AsyncBus<T> {
    bus: sync::Bus<T>,
    tasks: Vec<Arc<AtomicTask>>,
    sink_closed: Arc<AtomicBool>,
    task_receiver: Receiver<Arc<AtomicTask>>,
}

pub struct AsyncBusReader<T> {
    reader: sync::BusReader<T>,
    task: Arc<AtomicTask>,
    sink_closed: Arc<AtomicBool>,
    task_sender: Sender<Arc<AtomicTask>>,
}

pub fn channel<T: Send>(size: usize) -> (AsyncBus<T>, AsyncBusReader<T>) {
    let (sync_bus, sync_bus_reader) = sync::channel(size);
    let (sender, receiver) = mpsc::channel();
    let closed = Arc::new(AtomicBool::new(false));
    let arc = Arc::new(AtomicTask::new());
    sender.send(arc.clone()).unwrap();
    (
        AsyncBus {
            bus: sync_bus,
            tasks: Vec::new(),
            task_receiver: receiver,
            sink_closed: closed.clone(),
        },
        AsyncBusReader {
            reader: sync_bus_reader,
            task: arc,
            sink_closed: closed.clone(),
            task_sender: sender,
        },
    )
}

impl<T> Sink for AsyncBus<T> {
    type SinkItem = T;
    type SinkError = ();

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        for task in self.task_receiver.try_recv().into_iter() {
            self.tasks.push(task);
        }
        self.bus.broadcast(item);
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

impl<T> Drop for AsyncBus<T> {
    fn drop(&mut self) {
        self.close().unwrap();
        self.poll_complete().unwrap();
    }
}

impl<T> Stream for AsyncBusReader<T> {
    type Item = Arc<T>;
    type Error = ();
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.reader.recv() {
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

impl<T> Clone for AsyncBusReader<T> {
    fn clone(&self) -> Self {
        let arc = Arc::new(AtomicTask::new());
        self.task_sender.send(arc.clone()).unwrap();
        Self {
            reader: self.reader.clone(),
            task: arc.clone(),
            task_sender: self.task_sender.clone(),
            sink_closed: self.sink_closed.clone(),
        }
    }
}
