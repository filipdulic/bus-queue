use crate::atomic_counter::AtomicCounter;
use crate::ring_buffer::{RingBuffer, TryRecvError};
use crate::swap_slot::SwapSlot;
use std::sync::Arc;

#[derive(Debug)]
pub struct Subscriber<T, S: SwapSlot<T>> {
    /// Shared reference to the channel
    pub(super) buffer: Arc<RingBuffer<T, S>>,
    /// Read index pointer
    pub(super) ri: AtomicCounter,
    /// how many items should the receiver skip when the writer overflows
    pub(super) skip_items: usize,
}

impl<T, S: SwapSlot<T>> From<Arc<RingBuffer<T, S>>> for Subscriber<T, S> {
    fn from(arc_channel: Arc<RingBuffer<T, S>>) -> Self {
        Self {
            buffer: arc_channel,
            skip_items: 0,
            ri: AtomicCounter::new(0),
        }
    }
}

impl<T, S: SwapSlot<T>> Subscriber<T, S> {
    /// Returns true if the sender is available, otherwise false
    #[allow(dead_code)]
    pub fn is_sender_available(&self) -> bool {
        self.buffer.is_available()
    }

    /// Sets the skip_items attribute of the reader to a max value being the queue size.
    #[allow(dead_code)]
    pub fn set_skip_items(&mut self, skip_items: usize) {
        self.skip_items = std::cmp::min(skip_items, self.buffer.len() - 1);
    }

    /// Receives some atomic reference to an object if queue is not empty, or None if it is. Never
    /// Blocks
    pub fn try_recv(&self) -> Result<Arc<T>, TryRecvError> {
        self.buffer.try_recv(&self.ri, self.skip_items)
    }

    /// Returns the length of the queue.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Checks if nothings has been published yet.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_sub_empty(self.ri.get())
    }
}

/// Clone trait is used to create a Receiver which receives messages from the same Sender
impl<T, S: SwapSlot<T>> Clone for Subscriber<T, S> {
    fn clone(&self) -> Self {
        self.buffer.inc_sub_count();
        Self {
            buffer: self.buffer.clone(),
            ri: AtomicCounter::new(self.ri.get()),
            skip_items: self.skip_items,
        }
    }
}

impl<T, S: SwapSlot<T>> Drop for Subscriber<T, S> {
    fn drop(&mut self) {
        self.buffer.dec_sub_count();
    }
}

impl<T, S: SwapSlot<T>> PartialEq for Subscriber<T, S> {
    fn eq(&self, other: &Subscriber<T, S>) -> bool {
        Arc::ptr_eq(&self.buffer, &other.buffer) && self.ri == other.ri
    }
}

impl<T, S: SwapSlot<T>> Eq for Subscriber<T, S> {}

impl<T, S: SwapSlot<T>> Iterator for Subscriber<T, S> {
    type Item = Arc<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.try_recv().ok()
    }
}
