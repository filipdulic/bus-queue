use crate::atomic_counter::AtomicCounter;
use crate::channel::{Channel, TryRecvError};
use crate::swap_slot::SwapSlot;
use std::sync::Arc;

#[derive(Debug)]
pub struct Receiver<T, S: SwapSlot<T>> {
    /// Shared reference to the channel
    pub(super) channel: Arc<Channel<T, S>>,
    /// Read index pointer
    pub(super) ri: AtomicCounter,
    /// how many items should the receiver skip when the writer overflows
    pub(super) skip_items: usize,
}

impl<T, S: SwapSlot<T>> From<Arc<Channel<T, S>>> for Receiver<T, S> {
    fn from(arc_channel: Arc<Channel<T, S>>) -> Self {
        Self {
            channel: arc_channel,
            skip_items: 0,
            ri: AtomicCounter::new(0),
        }
    }
}

impl<T, S: SwapSlot<T>> Receiver<T, S> {
    /// Returns true if the sender is available, otherwise false
    #[allow(dead_code)]
    pub fn is_sender_available(&self) -> bool {
        self.channel.is_available()
    }

    /// Sets the skip_items attribute of the reader to a max value being the queue size.
    #[allow(dead_code)]
    pub fn set_skip_items(&mut self, skip_items: usize) {
        self.skip_items = std::cmp::min(skip_items, self.channel.len() - 1);
    }

    /// Receives some atomic reference to an object if queue is not empty, or None if it is. Never
    /// Blocks
    pub fn try_recv(&self) -> Result<Arc<T>, TryRecvError> {
        self.channel.try_recv(&self.ri, self.skip_items)
    }

    /// Returns the length of the queue.
    pub fn len(&self) -> usize {
        self.channel.len()
    }

    /// Checks if nothings has been published yet.
    pub fn is_empty(&self) -> bool {
        self.channel.wi.get() == self.ri.get()
    }
}

/// Clone trait is used to create a Receiver which receives messages from the same Sender
impl<T, S: SwapSlot<T>> Clone for Receiver<T, S> {
    fn clone(&self) -> Self {
        self.channel.inc_sub_count();
        Self {
            channel: self.channel.clone(),
            ri: AtomicCounter::new(self.ri.get()),
            skip_items: self.skip_items,
        }
    }
}

impl<T, S: SwapSlot<T>> Drop for Receiver<T, S> {
    fn drop(&mut self) {
        self.channel.dec_sub_count();
    }
}

impl<T, S: SwapSlot<T>> PartialEq for Receiver<T, S> {
    fn eq(&self, other: &Receiver<T, S>) -> bool {
        Arc::ptr_eq(&self.channel, &other.channel) && self.ri == other.ri
    }
}

impl<T, S: SwapSlot<T>> Eq for Receiver<T, S> {}

impl<T, S: SwapSlot<T>> Iterator for Receiver<T, S> {
    type Item = Arc<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.try_recv().ok()
    }
}
