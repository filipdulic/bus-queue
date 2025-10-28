use crate::ring_buffer::{RingBuffer, SendError};
use crate::swap_slot::SwapSlot;
use std::sync::Arc;

#[derive(Debug)]
pub struct Publisher<T, I, S: SwapSlot<T, I>> {
    /// Shared reference to the channel
    pub(super) buffer: Arc<RingBuffer<T, I, S>>,
}

impl<T, I, S: SwapSlot<T, I>> Publisher<T, I, S> {
    /// Publishes values to the circular buffer at wi % size
    ///
    /// # Arguments
    /// * `object` - owned object to be published
    pub fn broadcast(&self, object: I) -> Result<(), SendError<I>> {
        self.buffer.broadcast(object)
    }

    /// Returns the length of the queue
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Checks if nothings has been published yet
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Closes the Sender
    pub fn close(&self) {
        self.buffer.close()
    }
}

impl<T, I, S: SwapSlot<T, I>> From<Arc<RingBuffer<T, I, S>>> for Publisher<T, I,S> {
    fn from(arc_channel: Arc<RingBuffer<T,I, S>>) -> Self {
        Self {
            buffer: arc_channel,
        }
    }
}

/// Drop trait is used to let subscribers know that publisher is no longer available.
impl<T, I, S: SwapSlot<T, I>> Drop for Publisher<T, I, S> {
    fn drop(&mut self) {
        self.close();
    }
}

impl<T, I, S: SwapSlot<T, I>> PartialEq for Publisher<T, I, S> {
    fn eq(&self, other: &Publisher<T, I, S>) -> bool {
        Arc::ptr_eq(&self.buffer, &other.buffer)
    }
}

impl<T, I, S: SwapSlot<T, I>> Eq for Publisher<T, I, S> {}
