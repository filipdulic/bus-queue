use crate::piper::event::{Event, EventListener};
use crate::ring_buffer::TryRecvError;
use crate::subscriber::GenericSubscriber;
use crate::swap_slot::SwapSlot;
//use piper::{Event, EventListener};
use futures_core::{
    future::Future,
    task::{self, Poll},
    Stream,
};
use std::pin::Pin;
use std::sync::Arc;

pub struct GenericAsyncSubscriber<T, S: SwapSlot<T>> {
    pub(super) subscriber: GenericSubscriber<T, S>,
    pub(super) event: Arc<Event>,
    pub(super) listener: Option<EventListener>,
}

impl<T, S: SwapSlot<T>> From<(GenericSubscriber<T, S>, Arc<Event>)>
    for GenericAsyncSubscriber<T, S>
{
    fn from(input: (GenericSubscriber<T, S>, Arc<Event>)) -> Self {
        Self {
            subscriber: input.0,
            event: input.1,
            listener: None,
        }
    }
}

impl<T, S: SwapSlot<T>> std::fmt::Debug for GenericAsyncSubscriber<T, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscriber").finish()
    }
}

impl<T, S: SwapSlot<T>> GenericAsyncSubscriber<T, S> {
    #[allow(dead_code)]
    pub fn set_skip_items(&mut self, skip_items: usize) {
        self.subscriber.set_skip_items(skip_items);
    }

    /// Returns the number of remaining in the stream.
    pub fn len(&self) -> usize {
        self.subscriber.len()
    }

    /// Checks if stream is empty.
    pub fn is_empty(&self) -> bool {
        self.subscriber.is_empty()
    }
}

impl<T, S: SwapSlot<T>> Stream for GenericAsyncSubscriber<T, S> {
    type Item = Arc<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            // If this stream is blocked on an event, first make sure it is unblocked.
            if let Some(listener) = self.listener.as_mut() {
                futures_core::ready!(Pin::new(listener).poll(cx));
                self.listener = None;
            }
            loop {
                // Attempt to receive a message.
                match self.subscriber.try_recv() {
                    Ok(item) => {
                        // The stream is not blocked on an event - drop the listener.
                        self.listener = None;
                        return Poll::Ready(Some(item));
                    }
                    Err(TryRecvError::Disconnected) => {
                        // The stream is not blocked on an event - drop the listener.
                        self.listener = None;
                        return Poll::Ready(None);
                    }
                    Err(TryRecvError::Empty) => {}
                }
                // Listen for a send event.
                match self.listener.as_mut() {
                    None => {
                        // Store a listener and try sending the message again.
                        self.listener = Some(self.event.listen())
                    }
                    Some(_) => {
                        // Go back to the outer loop to poll the listener.
                        break;
                    }
                }
            }
        }
    }
}

impl<T, S: SwapSlot<T>> Clone for GenericAsyncSubscriber<T, S> {
    fn clone(&self) -> Self {
        Self {
            subscriber: self.subscriber.clone(),
            event: self.event.clone(),
            listener: None,
        }
    }
}

impl<T, S: SwapSlot<T>> PartialEq for GenericAsyncSubscriber<T, S> {
    fn eq(&self, other: &GenericAsyncSubscriber<T, S>) -> bool {
        self.subscriber == other.subscriber
    }
}

impl<T, S: SwapSlot<T>> Eq for GenericAsyncSubscriber<T, S> {}
