use crate::channel::{Receiver, TryRecvError};
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

pub struct Subscriber<T, S: SwapSlot<T>> {
    pub(super) receiver: Receiver<T, S>,
    pub(super) event: Arc<Event>,
    pub(super) listener: Option<EventListener>,
}

impl<T, S: SwapSlot<T>> From<(Receiver<T, S>, Arc<Event>)> for Subscriber<T, S> {
    fn from(input: (Receiver<T, S>, Arc<Event>)) -> Self {
        Self {
            receiver: input.0,
            event: input.1,
            listener: None,
        }
    }
}

impl<T, S: SwapSlot<T>> std::fmt::Debug for Subscriber<T, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscriber").finish()
    }
}

impl<T, S: SwapSlot<T>> Subscriber<T, S> {
    #[allow(dead_code)]
    pub fn set_skip_items(&mut self, skip_items: usize) {
        self.receiver.set_skip_items(skip_items);
    }
}

impl<T, S: SwapSlot<T>> Stream for Subscriber<T, S> {
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

impl<T, S: SwapSlot<T>> Clone for Subscriber<T, S> {
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver.clone(),
            event: self.event.clone(),
            listener: None,
        }
    }
}

impl<T, S: SwapSlot<T>> PartialEq for Subscriber<T, S> {
    fn eq(&self, other: &Subscriber<T, S>) -> bool {
        self.receiver == other.receiver
    }
}

impl<T, S: SwapSlot<T>> Eq for Subscriber<T, S> {}
