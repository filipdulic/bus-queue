use super::{Arc, Ordering::Relaxed};
use super::{Bus, BusReader};
use futures::prelude::*;
use futures::task::{current, Task};
use futures::AsyncSink;
use futures::{Async::NotReady, Async::Ready};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Receiver, Sender};

pub struct AsyncBusReader<T> {
    reader: BusReader<T>,
    task_sender: Sender<Task>,
    sink_closed: Arc<AtomicBool>,
}

impl<T> Stream for AsyncBusReader<T> {
    type Item = Arc<T>;
    type Error = ();
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.reader.recv() {
            Some(arc_object) => Ok(Ready(Some(arc_object))),
            None => {
                if self.sink_closed.load(Relaxed) == true {
                    // Stream closed no further data will be available ever
                    println!("Ready None sent");
                    Ok(Ready(None))
                } else {
                    self.task_sender.send(current()).unwrap();
                    Ok(NotReady)
                }
            }
        }
    }
}

//impl<T> Drop for AsyncBusReader<T>{
//    fn drop(&mut self){}
//}

pub struct AsyncBus<T> {
    bus: Bus<T>,
    task_sender: Sender<Task>,
    task_receiver: Receiver<Task>,
    sink_closed: Arc<AtomicBool>,
}

impl<T> AsyncBus<T> {
    pub fn new(size: usize) -> Self {
        let (tx, rx) = channel();
        Self {
            bus: Bus::new(size),
            task_receiver: rx,
            task_sender: tx,
            sink_closed: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn add_sub(&mut self) -> AsyncBusReader<T> {
        AsyncBusReader {
            reader: self.bus.add_sub(),
            task_sender: self.task_sender.clone(),
            sink_closed: self.sink_closed.clone(),
        }
    }
    pub fn push(&mut self,object:T){
        self.bus.push(object);
        for t in self.task_receiver.try_recv() {
            t.notify();
        }
    }
}

impl<T> Sink for AsyncBus<T> {
    type SinkItem = T;
    type SinkError = ();

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.bus.push(item);
        Ok(AsyncSink::Ready)
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        for task in self.task_receiver.try_recv() {
            task.notify();
        }
        Ok(Ready(()))
    }
    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.sink_closed.store(true, Relaxed);
        Ok(Ready(()))
    }
}

//impl<T> Drop for AsyncBus<T>{
//    fn drop(&mut self){
//        self.poll_complete().unwrap();
//    }
//}
