use super::{Arc, Ordering::Relaxed};
use super::{Bus, BusReader};
use super::arc_swap::ArcSwap;
use futures::prelude::*;
use futures::task::AtomicTask;
use futures::AsyncSink;
use futures::{Async::NotReady, Async::Ready};
use std::sync::atomic::AtomicBool;

pub struct AsyncBusReader<T> {
    reader: BusReader<T>,
    task: Arc<AtomicTask>,
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
                    self.task.register();
                    Ok(NotReady)
                }
            }
        }
    }
}

pub struct AsyncBus<T> {
    bus: Bus<T>,
    tasks: Vec<ArcSwap<AtomicTask>>,
    sink_closed: Arc<AtomicBool>,
}

impl<T> AsyncBus<T> {
    pub fn new(size: usize) -> Self {
        Self {
            bus: Bus::new(size),
            tasks: Vec::new(),
            sink_closed: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn add_sub(&mut self) -> AsyncBusReader<T> {
        let arc = Arc::new(AtomicTask::new());
        self.tasks.push(ArcSwap::from(arc.clone()));
        AsyncBusReader {
            reader: self.bus.add_sub(),
            task: arc.clone(),
            sink_closed: self.sink_closed.clone(),
        }
    }
    pub fn push(&mut self,object:T){
        self.bus.push(object);
        for t in &self.tasks{
            t.load().notify();
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
        for t in &self.tasks{
            t.load().notify();
        }
        Ok(Ready(()))
    }
    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.sink_closed.store(true, Relaxed);
        Ok(Ready(()))
    }
}

