use crate::piper::event::Event;
use crate::publisher::GenericPublisher;
use crate::ring_buffer::SendError;
use crate::swap_slot::SwapSlot;
// use piper::Event;
use futures_core::task::{self, Poll};
use futures_sink::Sink;
use std::pin::Pin;
use std::sync::Arc;

pub struct GenericAsyncPublisher<T, S: SwapSlot<T>> {
    pub(super) sender: GenericPublisher<T, S>,
    pub(super) event: Arc<Event>,
}

impl<T, S: SwapSlot<T>> From<(GenericPublisher<T, S>, Arc<Event>)> for GenericAsyncPublisher<T, S> {
    fn from(input: (GenericPublisher<T, S>, Arc<Event>)) -> Self {
        Self {
            sender: input.0,
            event: input.1,
        }
    }
}

impl<T, S: SwapSlot<T>> Sink<T> for GenericAsyncPublisher<T, S> {
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

impl<T, S: SwapSlot<T>> PartialEq for GenericAsyncPublisher<T, S> {
    fn eq(&self, other: &GenericAsyncPublisher<T, S>) -> bool {
        self.sender == other.sender
    }
}

impl<T, S: SwapSlot<T>> Drop for GenericAsyncPublisher<T, S> {
    fn drop(&mut self) {
        self.sender.close();
        self.event.notify_all();
    }
}

impl<T, S: SwapSlot<T>> Eq for GenericAsyncPublisher<T, S> {}
