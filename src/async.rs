use super::{Bus, BusReader, Ordering::Relaxed};
use futures::prelude::*;
use futures::{task::AtomicTask, Async, AsyncSink};
use std::sync::{atomic::AtomicBool, mpsc, mpsc::Receiver, mpsc::Sender, Arc};

pub struct AsyncBusReader<T> {
    reader: BusReader<T>,
    task: Arc<AtomicTask>,
    sink_closed: Arc<AtomicBool>,
    task_sender: Sender<Arc<AtomicTask>>,
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

pub struct AsyncBus<T> {
    bus: Bus<T>,
    tasks: Vec<Arc<AtomicTask>>,
    sink_closed: Arc<AtomicBool>,
    task_receiver: Receiver<Arc<AtomicTask>>,
}

impl<T> AsyncBus<T> {
    fn new(size: usize, receiver: Receiver<Arc<AtomicTask>>) -> Self {
        Self {
            bus: Bus::new(size),
            tasks: Vec::new(),
            sink_closed: Arc::new(AtomicBool::new(false)),
            task_receiver: receiver,
        }
    }
    fn add_sub(&mut self, sender: Sender<Arc<AtomicTask>>) -> AsyncBusReader<T> {
        let arc = Arc::new(AtomicTask::new());
        self.tasks.push(arc.clone());
        AsyncBusReader {
            reader: self.bus.add_sub(),
            task: arc.clone(),
            sink_closed: self.sink_closed.clone(),
            task_sender: sender,
        }
    }
}

impl<T> Sink for AsyncBus<T> {
    type SinkItem = T;
    type SinkError = ();

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        for task in self.task_receiver.try_recv().into_iter() {
            self.tasks.push(task);
        }
        self.bus.push(item);
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

pub fn channel<T: Send>(size: usize) -> (AsyncBus<T>, AsyncBusReader<T>) {
    let (tx, rx) = mpsc::channel();
    let mut bus = AsyncBus::new(size, rx);
    let bus_reader = bus.add_sub(tx);
    (bus, bus_reader)
}
