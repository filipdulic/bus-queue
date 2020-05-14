use crate::atomic_counter::AtomicCounter;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
// Use std mpsc's error types as our own
use crate::swap_slot::SwapSlot;
use std::fmt::Debug;
pub use std::sync::mpsc::{RecvError, RecvTimeoutError, SendError, TryRecvError};

#[derive(Debug)]
pub struct RingBuffer<T, S: SwapSlot<T>> {
    /// Circular buffer
    buffer: Vec<S>,
    /// Size of the buffer
    size: usize,
    /// Write index pointer
    wi: AtomicCounter,
    /// Number of subscribers
    sub_count: AtomicCounter,
    /// true if this sender is still available
    is_available: AtomicBool,
    ph: std::marker::PhantomData<T>,
}

impl<T, S: SwapSlot<T>> RingBuffer<T, S> {
    pub fn new(size: usize) -> Self {
        let size = size + 1;
        let mut buffer = Vec::with_capacity(size);
        for _i in 0..size {
            buffer.push(S::none())
        }
        Self {
            buffer,
            size,
            wi: AtomicCounter::new(0),
            sub_count: AtomicCounter::new(1),
            is_available: AtomicBool::new(true),
            ph: std::marker::PhantomData,
        }
    }
    /// Publishes values to the circular buffer at wi % size
    ///
    /// # Arguments
    /// * `object` - owned object to be published
    pub fn broadcast(&self, object: T) -> Result<(), SendError<T>> {
        if self.sub_count.get() == 0 {
            return Err(SendError(object));
        }
        self.buffer[self.wi.get() % self.size].store(object);
        self.wi.inc();
        Ok(())
    }

    /// Receives some atomic reference to an object if queue is not empty, or None if it is. Never
    /// Blocks
    pub fn try_recv(&self, ri: &AtomicCounter, skip_items: usize) -> Result<Arc<T>, TryRecvError> {
        if ri.get() == self.wi.get() {
            if self.is_available() {
                return Err(TryRecvError::Empty);
            } else {
                return Err(TryRecvError::Disconnected);
            }
        }

        // Reader has not read enough to keep up with (writer - buffer size) so
        // set the reader pointer to be (writer - buffer size)
        loop {
            let local_ri = ri.get();

            let val = self.buffer[local_ri % self.size].load();
            if self.wi.get().wrapping_sub(local_ri) >= self.size {
                ri.set(
                    self.wi
                        .get()
                        .wrapping_sub(self.size)
                        .wrapping_add(1 + skip_items),
                );
            } else {
                ri.inc();
                // NOTE: unwrap is safe to use, because the reader would never read a slot that
                // hasn't been written to.
                return Ok(val.unwrap());
            }
        }
    }

    /// Closes the channel
    pub fn close(&self) {
        self.is_available.store(false, Ordering::Relaxed);
    }
    /// Returns true if the sender is available, otherwise false
    pub fn is_available(&self) -> bool {
        self.is_available.load(Ordering::Relaxed)
    }

    /// Returns the length of the queue
    pub fn len(&self) -> usize {
        self.size - 1
    }

    /// Checks if nothings has been published yet
    pub fn is_empty(&self) -> bool {
        self.wi.get() == 0
    }

    /// Checks if subscriber has read all published items
    pub fn is_sub_empty(&self, ri: usize) -> bool {
        self.wi.get() == ri
    }

    /// Increment the number of subs
    pub fn inc_sub_count(&self) {
        self.sub_count.inc();
    }

    /// Decrement the number of subs
    pub fn dec_sub_count(&self) {
        self.sub_count.dec();
    }
}

/// Drop trait is used to let subscribers know that publisher is no longer available.
impl<T, S: SwapSlot<T>> Drop for RingBuffer<T, S> {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod test {
    use super::SwapSlot;
    use crate::flavors::arc_swap::bounded;
    use crate::ring_buffer::TryRecvError;

    #[test]
    fn subcount() {
        let (sender, receiver) = bounded::<()>(1);
        let receiver2 = receiver.clone();
        assert_eq!(sender.buffer.sub_count.get(), 2);
        assert_eq!(receiver.buffer.sub_count.get(), 2);
        assert_eq!(receiver2.buffer.sub_count.get(), 2);
        drop(receiver2);

        assert_eq!(sender.buffer.sub_count.get(), 1);
        assert_eq!(receiver.buffer.sub_count.get(), 1);
    }

    #[test]
    fn bounded_channel() {
        let (sender, receiver) = bounded::<i32>(1);
        let receiver2 = receiver.clone();
        sender.broadcast(123).unwrap();
        assert_eq!(*receiver.try_recv().unwrap(), 123);
        assert_eq!(*receiver2.try_recv().unwrap(), 123);
    }

    #[test]
    fn bounded_channel_no_subs() {
        let (sender, receiver) = bounded(1);
        drop(receiver);
        let err = sender.broadcast(123);
        assert!(err.is_err());
    }

    #[test]
    fn bounded_channel_no_sender() {
        let (sender, receiver) = bounded::<()>(1);
        drop(sender);
        assert_eq!(receiver.is_sender_available(), false);
    }

    #[test]
    fn bounded_channel_size() {
        let (sender, receiver) = bounded::<()>(3);
        assert_eq!(sender.len(), 3);
        assert_eq!(receiver.len(), 3);
    }

    #[test]
    fn bounded_within_size() {
        let (sender, receiver) = bounded(3);
        assert_eq!(sender.len(), 3);

        for i in 0..3 {
            sender.broadcast(i).unwrap();
        }

        let values = receiver.into_iter().map(|v| *v).collect::<Vec<_>>();
        assert_eq!(values, (0..=2).collect::<Vec<i32>>());
    }

    #[test]
    fn bounded_overflow() {
        let (sender, receiver) = bounded(3);
        assert_eq!(sender.len(), 3);

        for i in 0..4 {
            sender.broadcast(i).unwrap();
        }

        let values = receiver.into_iter().map(|v| *v).collect::<Vec<_>>();
        assert_eq!(values, (1..=3).collect::<Vec<i32>>());
    }

    #[test]
    fn bounded_overflow_with_reads() {
        let (sender, receiver) = bounded(3);
        assert_eq!(sender.len(), 3);

        for i in 0..3 {
            sender.broadcast(i).unwrap();
        }

        assert_eq!(*receiver.try_recv().unwrap(), 0);
        assert_eq!(*receiver.try_recv().unwrap(), 1);

        // "Cycle" buffer around twice
        for i in 3..10 {
            sender.broadcast(i).unwrap();
        }

        // Should be reading from the last element in the buffer
        let index = (receiver.buffer.wi.get() - receiver.buffer.size + 1) % receiver.buffer.size;

        assert_eq!(*SwapSlot::load(&receiver.buffer.buffer[index]).unwrap(), 7);
        assert_eq!(*receiver.try_recv().unwrap(), 7);

        // Cloned receiver start reading where the original receiver left off
        let receiver2 = receiver.clone();
        assert_eq!(*receiver2.try_recv().unwrap(), 8);
        assert_eq!(*receiver2.try_recv().unwrap(), 9);
        assert_eq!(receiver2.try_recv(), Err(TryRecvError::Empty));

        sender.broadcast(10).unwrap();

        // Test reader has moved forward in the buffer
        let values = receiver.into_iter().map(|v| *v).collect::<Vec<_>>();
        assert_eq!(values, (8..=10).collect::<Vec<i32>>());
    }

    #[test]
    fn read_before_writer_increments() {
        let (sender, receiver) = bounded(3);
        assert_eq!(sender.len(), 3);

        for i in 0..3 {
            sender.broadcast(i).unwrap();
        }
        assert_eq!(sender.buffer.wi.get(), 3);
        assert_eq!(receiver.ri.get(), 0);

        // Inserts the value 3, but does not increment the index.
        SwapSlot::store(
            &sender.buffer.buffer[sender.buffer.wi.get() % sender.buffer.size],
            3,
        );
        // Receiver still expects the oldest value in buffer to be returned.
        assert_eq!(*receiver.try_recv().unwrap(), 0);
        // reset receiver index
        receiver.ri.set(0);

        // sender index is incremented
        sender.buffer.wi.inc();
        assert_eq!(*receiver.try_recv().unwrap(), 1);

        // reset receiver index
        receiver.ri.set(0);

        // Inserts the value 4, but does not increment the index.
        SwapSlot::store(
            &sender.buffer.buffer[sender.buffer.wi.get() % sender.buffer.size],
            4,
        );
        // Receiver still expects the oldest value in buffer to be returned.
        assert_eq!(*receiver.try_recv().unwrap(), 1);
    }

    #[test]
    fn writer_overflows_pass_usize_max_less_then_size() {
        let (sender, receiver) = bounded(3);
        // set Sender wi index to usize::MAX - 3
        sender.buffer.wi.set(usize::max_value() - 3);
        // fill buffer so that reader can read oldest value in buffer (1,2,3)
        for i in 1..4 {
            sender.broadcast(i).unwrap();
        }
        assert_eq!(*receiver.try_recv().unwrap(), 1);
        assert_eq!(*receiver.try_recv().unwrap(), 2);

        // wi should be at usize::max_value()
        assert_eq!(sender.buffer.wi.get(), usize::max_value());
        // ri should be at usize::max_value() -1
        assert_eq!(receiver.ri.get(), usize::max_value() - 1);

        // broadcast 2 more items (4,5) so wi is at 1
        for i in 4..6 {
            sender.broadcast(i).unwrap();
        }
        assert_eq!(sender.buffer.wi.get(), 1);
        // receiver should be able to receive 3
        assert_eq!(*receiver.try_recv().unwrap(), 3);
        // ri should be at usize::max_value()
        assert_eq!(receiver.ri.get(), usize::max_value());
    }

    #[test]
    fn writer_overflows_pass_usize_max_more_then_size() {
        let (sender, receiver) = bounded(3);
        // set Sender wi index to usize::MAX - 3
        sender.buffer.wi.set(usize::max_value() - 3);
        // fill buffer so that reader can read oldest value in buffer (1,2,3)
        for i in 1..4 {
            sender.broadcast(i).unwrap();
        }
        assert_eq!(*receiver.try_recv().unwrap(), 1);
        assert_eq!(*receiver.try_recv().unwrap(), 2);

        // wi should be at usize::max_value()
        assert_eq!(sender.buffer.wi.get(), usize::max_value());
        // ri should be at usize::max_value() -1
        assert_eq!(receiver.ri.get(), usize::max_value() - 1);

        // broadcast 6 more items (4,5,6,7,8,9) so wi is at 5
        for i in 4..10 {
            sender.broadcast(i).unwrap();
        }
        assert_eq!(sender.buffer.wi.get(), 5);

        // before calling try_recv() ri should be at usize::max_value() - 1
        assert_eq!(receiver.ri.get(), usize::max_value() - 1);
        // receiver should be able to receive 7
        assert_eq!(*receiver.try_recv().unwrap(), 7);
        // ri should be updated to 3
        assert_eq!(receiver.ri.get(), 3);
    }

    #[test]
    fn test_arc() {
        use std::sync::Arc;
        // make a sender with 2 receiver clones
        let (sender, receiver) = bounded(1);
        let receiver2 = receiver.clone();

        // Broadcast an item.
        // It is stored through an Arc inside the buffer
        // it's reference count is 1.
        sender.broadcast(1).unwrap();

        // Pick up the item through one receiver
        let arc1 = receiver.try_recv().unwrap();
        assert_eq!(*arc1, 1);
        // it's reference count jumps to 2.
        assert_eq!(Arc::strong_count(&arc1), 2);

        // Pick up the same item through the second receiver
        let arc2 = receiver2.try_recv().unwrap();
        // it's reference count jumps to 3.
        assert_eq!(Arc::strong_count(&arc2), 3);
        // the first received Arc ref count also jumps to 3.
        assert_eq!(Arc::strong_count(&arc1), 3);

        // Broadcast another item.
        // Since the internal buffer is actually bigger by 1 then the size
        // parameter sent the the bounded function, the item we published first
        // is still inside the buffer and it's reference counts is unchanged.
        sender.broadcast(2).unwrap();
        assert_eq!(Arc::strong_count(&arc1), 3);
        assert_eq!(Arc::strong_count(&arc2), 3);

        // By broadcasting another item, we have overwritten the first item
        // in the buffer and it's ref should drop by one.
        sender.broadcast(3).unwrap();
        assert_eq!(Arc::strong_count(&arc1), 2);
        assert_eq!(Arc::strong_count(&arc2), 2);
    }

    #[test]
    fn test_is_empty() {
        let (sender, receiver) = bounded(1);
        assert!(sender.is_empty());
        assert!(receiver.is_empty());
        assert!(sender.buffer.is_empty());
        sender.broadcast(1).unwrap();
        assert!(!sender.is_empty());
        assert!(!receiver.is_empty());
        assert!(!sender.buffer.is_empty());
    }

    #[test]
    fn test_sender_eq() {
        let (sender1, _) = bounded::<i32>(1);
        let (sender2, _) = bounded::<i32>(1);
        assert!(!sender1.eq(&sender2));
        assert!(sender1.eq(&sender1));
        assert!(sender2.eq(&sender2));
    }

    #[test]
    fn test_set_skip_items() {
        let (sender, receiver1) = bounded(3);
        let mut receiver2 = receiver1.clone();
        let mut receiver3 = receiver1.clone();
        let mut receiver4 = receiver1.clone();
        receiver2.set_skip_items(1);
        receiver3.set_skip_items(2);
        receiver4.set_skip_items(3);

        for i in 0..6 {
            sender.broadcast(i).unwrap();
        }
        assert_eq!(*receiver1.try_recv().unwrap(), 3);
        assert_eq!(*receiver2.try_recv().unwrap(), 4);
        assert_eq!(*receiver3.try_recv().unwrap(), 5);
        assert_eq!(*receiver4.try_recv().unwrap(), 5);
    }
}
