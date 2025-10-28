use crate::publisher::Publisher;
use crate::ring_buffer::SendError;
use crate::swap_slot::SwapSlot;
// use piper::Event;
use event_listener::Event;
use futures_core::task::{self, Poll};
use futures_sink::Sink;
use std::pin::Pin;
use std::sync::Arc;

pub struct AsyncPublisher<T, I, S: SwapSlot<T, I>> {
    pub(super) publisher: Publisher<T, I, S>,
    pub(super) event: Arc<Event>,
}

impl<T, I, S: SwapSlot<T, I>> From<(Publisher<T, I, S>, Arc<Event>)> for AsyncPublisher<T, I, S> {
    fn from(input: (Publisher<T, I, S>, Arc<Event>)) -> Self {
        Self {
            publisher: input.0,
            event: input.1,
        }
    }
}

impl<T, I, S: SwapSlot<T, I>> Sink<I> for AsyncPublisher<T, I, S> {
    type Error = SendError<I>;

    fn poll_ready(
        self: Pin<&mut Self>,
        _: &mut task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: I) -> Result<(), Self::Error> {
        self.publisher.broadcast(item).and_then(|_| Ok(()))
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
        self.publisher.close();
        self.poll_flush(cx)
    }
}

impl<T, I, S: SwapSlot<T, I>> PartialEq for AsyncPublisher<T, I, S> {
    fn eq(&self, other: &AsyncPublisher<T, I, S>) -> bool {
        self.publisher == other.publisher
    }
}

impl<T, I, S: SwapSlot<T, I>> Drop for AsyncPublisher<T, I, S> {
    fn drop(&mut self) {
        self.publisher.close();
        self.event.notify_all();
    }
}

impl<T, I, S: SwapSlot<T, I>> Eq for AsyncPublisher<T, I, S> {}
