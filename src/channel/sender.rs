use crate::channel::{Channel, SendError};
use crate::swap_slot::SwapSlot;
use std::sync::Arc;

#[derive(Debug)]
pub struct Sender<T, S: SwapSlot<T>> {
    /// Shared reference to the channel
    pub(super) channel: Arc<Channel<T, S>>,
}

impl<T, S: SwapSlot<T>> Sender<T, S> {
    /// Publishes values to the circular buffer at wi % size
    ///
    /// # Arguments
    /// * `object` - owned object to be published
    pub fn broadcast(&self, object: T) -> Result<(), SendError<T>> {
        self.channel.broadcast(object)
    }

    /// Returns the length of the queue
    pub fn len(&self) -> usize {
        self.channel.len()
    }

    /// Checks if nothings has been published yet
    pub fn is_empty(&self) -> bool {
        self.channel.wi.get() == 0
    }

    /// Closes the Sender
    pub fn close(&self) {
        self.channel.close()
    }
}

impl<T, S: SwapSlot<T>> From<Arc<Channel<T, S>>> for Sender<T, S> {
    fn from(arc_channel: Arc<Channel<T, S>>) -> Self {
        Self {
            channel: arc_channel,
        }
    }
}

/// Drop trait is used to let subscribers know that publisher is no longer available.
impl<T, S: SwapSlot<T>> Drop for Sender<T, S> {
    fn drop(&mut self) {
        self.close();
    }
}

impl<T, S: SwapSlot<T>> PartialEq for Sender<T, S> {
    fn eq(&self, other: &Sender<T, S>) -> bool {
        Arc::ptr_eq(&self.channel, &other.channel)
    }
}

impl<T, S: SwapSlot<T>> Eq for Sender<T, S> {}
