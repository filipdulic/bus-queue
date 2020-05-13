use crate::channel::{Subscriber, TryRecvError};
use crate::piper::event::{Event, EventListener};
use crate::swap_slot::SwapSlot;
//use piper::{Event, EventListener};
use futures_core::{
    future::Future,
    task::{self, Poll},
    Stream,
};
use std::pin::Pin;
use std::sync::Arc;

pub struct AsyncSubscriber<T, S: SwapSlot<T>> {
    pub(super) receiver: Subscriber<T, S>,
    pub(super) event: Arc<Event>,
    pub(super) listener: Option<EventListener>,
}

impl<T, S: SwapSlot<T>> From<(Subscriber<T, S>, Arc<Event>)> for AsyncSubscriber<T, S> {
    fn from(input: (Subscriber<T, S>, Arc<Event>)) -> Self {
        Self {
            receiver: input.0,
            event: input.1,
            listener: None,
        }
    }
}

impl<T, S: SwapSlot<T>> std::fmt::Debug for AsyncSubscriber<T, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscriber").finish()
    }
}

impl<T, S: SwapSlot<T>> AsyncSubscriber<T, S> {
    #[allow(dead_code)]
    pub fn set_skip_items(&mut self, skip_items: usize) {
        self.receiver.set_skip_items(skip_items);
    }

    /// Returns the number of remaining in the stream.
    pub fn len(&self) -> usize {
        self.receiver.len()
    }

    /// Checks if stream is empty.
    pub fn is_empty(&self) -> bool {
        self.receiver.is_empty()
    }
}

impl<T, S: SwapSlot<T>> Stream for AsyncSubscriber<T, S> {
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
                match self.receiver.try_recv() {
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

impl<T, S: SwapSlot<T>> Clone for AsyncSubscriber<T, S> {
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver.clone(),
            event: self.event.clone(),
            listener: None,
        }
    }
}

impl<T, S: SwapSlot<T>> PartialEq for AsyncSubscriber<T, S> {
    fn eq(&self, other: &AsyncSubscriber<T, S>) -> bool {
        self.receiver == other.receiver
    }
}

impl<T, S: SwapSlot<T>> Eq for AsyncSubscriber<T, S> {}
