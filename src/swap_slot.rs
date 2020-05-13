use std::sync::Arc;

/// Trait required by implementers of syncing primitives.
pub trait SwapSlot<T> {
    /// Creates a new Arc around item and stores it,
    /// dropping the previously held item's Arc.
    fn store(&self, item: T);

    /// Returns a clone of the held Arc,
    /// incrementing the ref count atomically
    fn load(&self) -> Option<Arc<T>>;

    /// Creates a placeholder without an item.
    /// Due to the queue's internal implementation
    /// placeholders are never read, only overwritten,
    /// but are required because of the bounded constraint.
    fn none() -> Self;
}
