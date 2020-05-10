use std::sync::Arc;

/// Trait required by implementers of syncing primitives.
pub trait SwapSlot<T> {
    /// Creates a new Arc around item and stores it,
    /// dropping the previously held item's Arc.
    fn store(&self, item: T);
    /// Returns a clone of the held Arc,
    /// incrementing the ref count atomically
    fn load(&self) -> Arc<T>;
    /// Creates a placeholder without an item.
    /// Due to the queue's internal implementation
    /// placeholders are never read, only overwritten,
    /// but are required because of the bounded constraint.
    fn none() -> Self;
}
#[cfg(test)]
mod test {
    // uses the libraries default exported slot
    use crate::swap_slot::SwapSlot;
    use crate::Slot;
    use std::sync::Arc;

    #[test]
    fn test_swap_slot() {
        // Initialize slot
        let item = Slot::none();

        // Store an item into the slot, ref count = 1
        SwapSlot::store(&item, 1);

        // Load an item from the slot, ref count = 2
        let arc = SwapSlot::load(&item);
        assert_eq!(*arc, 1);
        assert_eq!(Arc::strong_count(&arc), 2);

        // Store another item into the slot overwriting the previous item.
        // old item ref count drops to 1, new item ref count is 1.
        SwapSlot::store(&item, 2);
        assert_eq!(Arc::strong_count(&arc), 1);

        // Load new item from the slot, ref count 2
        let arc = SwapSlot::load(&item);
        assert_eq!(*arc, 2);
        assert_eq!(Arc::strong_count(&arc), 2);
    }
}
