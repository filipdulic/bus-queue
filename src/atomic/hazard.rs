//! This file was copied from https://github.com/stjepang/atomic
//! Original author "Stjepan Glavina <stjepang@gmail.com>", all rights are his
//! It will only be temporarily used until the project is published by the
//! original author on crates.io or until the author asks for it's removal.
use std::ptr;
use std::sync::atomic::{self, AtomicBool, AtomicPtr, AtomicUsize, Ordering};

// TODO: if needs_drop is false, add retiring object to a freelist, and use sizeof to track memory

pub fn allocate_slot() -> *const AtomicUsize {
    LOCAL.with(|local| local.allocate_slot() as *const AtomicUsize)
}

#[derive(Default)]
struct ThreadEntry {
    slots: [AtomicUsize; 6],
    next: AtomicPtr<ThreadEntry>,
    in_use: AtomicBool,
}

#[derive(Default)]
pub struct Registry {
    entries: [ThreadEntry; 32],
    next: AtomicPtr<Registry>,
}

static REGISTRY: AtomicPtr<Registry> = AtomicPtr::new(ptr::null_mut());

#[allow(clippy::cmp_null)]
fn try_extend_registry(ptr: &AtomicPtr<Registry>) {
    let instance = Box::into_raw(Box::new(Registry::default()));

    if ptr.compare_and_swap(ptr::null_mut(), instance, Ordering::SeqCst) != ptr::null_mut() {
        // Some other thread has successfully extended Registry.
        // It is our job now to delete `instance` we have just created.
        unsafe { drop(Box::from_raw(instance)) }
    }
}

#[inline]
pub fn registry() -> &'static Registry {
    let mut registry = REGISTRY.load(Ordering::Acquire);

    if registry.is_null() {
        try_extend_registry(&REGISTRY);
        registry = REGISTRY.load(Ordering::Acquire);
    }

    unsafe { &(*registry) }
}

impl Registry {
    #[cold]
    pub fn destroy_object(&self, obj: usize) -> bool {
        if obj == 0 {
            false
        } else {
            !self.try_transfer_drop_responsibility(obj)
        }
    }

    #[allow(clippy::collapsible_if)]
    fn register(&self) -> *const ThreadEntry {
        for entry in self.entries.iter() {
            if !entry.in_use.load(Ordering::SeqCst) {
                if !entry.in_use.swap(true, Ordering::SeqCst) {
                    return entry;
                }
            }
        }

        let mut next = self.next.load(Ordering::SeqCst);

        if next.is_null() {
            try_extend_registry(&self.next);
            next = self.next.load(Ordering::SeqCst);
        }

        unsafe { (*next).register() }
    }

    #[allow(clippy::collapsible_if)]
    fn try_transfer_drop_responsibility(&self, ptr: usize) -> bool {
        debug_assert_ne!(ptr, 0);

        atomic::fence(Ordering::SeqCst);

        for entry in self.entries.iter() {
            if entry.in_use.load(Ordering::Acquire) {
                if entry.try_transfer_drop_responsibility(ptr) {
                    return true;
                }
            }
        }

        unsafe {
            let next = self.next.load(Ordering::Acquire);

            if !next.is_null() {
                (*next).try_transfer_drop_responsibility(ptr)
            } else {
                false
            }
        }
    }
}

impl ThreadEntry {
    fn unregister(&self) {
        self.in_use.store(false, Ordering::SeqCst)
    }

    #[inline]
    fn allocate_slot(&self) -> *const AtomicUsize {
        for slot in self.slots.iter() {
            if slot.load(Ordering::Relaxed) == 0 {
                return slot;
            }
        }

        let mut next = self.next.load(Ordering::SeqCst);

        if next.is_null() {
            let new_entry = Box::into_raw(Box::new(ThreadEntry::default()));
            self.next.store(new_entry, Ordering::SeqCst);
            next = new_entry;
        }

        unsafe { (*next).allocate_slot() }
    }

    #[allow(clippy::needless_return)]
    #[allow(clippy::collapsible_if)]
    fn try_transfer_drop_responsibility(&self, ptr: usize) -> bool {
        debug_assert_ne!(ptr, 0);

        for slot in self.slots.iter() {
            if slot.load(Ordering::SeqCst) == ptr {
                if slot.compare_and_swap(ptr, 1, Ordering::SeqCst) == ptr {
                    return true;
                }
            }
        }
        return false;
    }
}

struct Local {
    entry: *const ThreadEntry,
}

thread_local! {
    static LOCAL: Local = Local::new();
}

impl Local {
    pub fn new() -> Self {
        Local {
            entry: registry().register(),
        }
    }

    #[inline]
    fn allocate_slot(&self) -> &AtomicUsize {
        unsafe { &*(*self.entry).allocate_slot() }
    }
}

impl Drop for Local {
    fn drop(&mut self) {
        unsafe { (*self.entry).unregister() }
    }
}
