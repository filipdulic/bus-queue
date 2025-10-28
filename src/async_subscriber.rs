use crate::ring_buffer::TryRecvError;
use crate::subscriber::Subscriber;
use crate::swap_slot::SwapSlot;
use event_listener::{Event, EventListener};
//use piper::{Event, EventListener};
use futures_core::{
    future::Future,
    task::{self, Poll},
    Stream,
};
use std::pin::Pin;
use std::sync::Arc;

pub struct AsyncSubscriber<T, I, S: SwapSlot<T, I>> {
    pub(super) subscriber: Subscriber<T, I, S>,
    pub(super) event: Arc<Event>,
    pub(super) listener: Option<EventListener>,
}

impl<T, I, S: SwapSlot<T, I>> From<(Subscriber<T, I, S>, Arc<Event>)> for AsyncSubscriber<T, I, S> {
    fn from(input: (Subscriber<T, I, S>, Arc<Event>)) -> Self {
        Self {
            subscriber: input.0,
            event: input.1,
            listener: None,
        }
    }
}

impl<T, I, S: SwapSlot<T, I>> std::fmt::Debug for AsyncSubscriber<T, I, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscriber").finish()
    }
}

impl<T, I, S: SwapSlot<T, I>> AsyncSubscriber<T, I, S> {
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

impl<T, I, S: SwapSlot<T, I>> Stream for AsyncSubscriber<T, I, S> {
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

impl<T, I, S: SwapSlot<T, I>> Clone for AsyncSubscriber<T, I, S> {
    fn clone(&self) -> Self {
        Self {
            subscriber: self.subscriber.clone(),
            event: self.event.clone(),
            listener: None,
        }
    }
}

impl<T, I, S: SwapSlot<T, I>> PartialEq for AsyncSubscriber<T, I, S> {
    fn eq(&self, other: &AsyncSubscriber<T, I, S>) -> bool {
        self.subscriber == other.subscriber
    }
}

impl<T, I, S: SwapSlot<T, I>> Eq for AsyncSubscriber<T, I, S> {}
