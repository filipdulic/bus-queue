#[cfg(feature = "arcswap")]
pub mod arc_swap;

#[cfg(feature = "rwlock")]
pub mod rw_lock;

#[cfg(test)]
mod allocation_tests {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    struct CountingAlloc;

    #[global_allocator]
    static GLOBAL: CountingAlloc = CountingAlloc;

    thread_local! {
        // Each OS thread gets its own counter.
        static ALLOCS_THIS_THREAD: Cell<usize> = const { Cell::new(0) };
    }

    unsafe impl GlobalAlloc for CountingAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let ptr = unsafe { System.alloc(layout) };
            if !ptr.is_null() {
                // Only bump the counter for the *current* thread.
                ALLOCS_THIS_THREAD.with(|c| c.set(c.get() + 1));
            }
            ptr
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe { System.dealloc(ptr, layout) };
        }
    }

    pub(crate) fn reset_allocs_current_thread() {
        ALLOCS_THIS_THREAD.with(|c| c.set(0));
    }

    pub(crate) fn allocs_current_thread() -> usize {
        ALLOCS_THIS_THREAD.with(|c| c.get())
    }
}
