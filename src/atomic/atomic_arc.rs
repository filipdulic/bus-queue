//! This file was copied from https://github.com/stjepang/atomic
//! Original author "Stjepan Glavina <stjepang@gmail.com>", all rights are his
//! It will only be temporarily used until the project is published by the
//! original author on crates.io or until the author asks for it's removal.
use std::marker::PhantomData;
use std::mem;
use std::ptr;
use std::sync::atomic::{self, AtomicPtr, AtomicUsize, Ordering};
use std::sync::Arc;

use super::hazard;

// TODO: ZSV are all allocated at address 0x1 - what about transfering drop resp.?
// - maybe we should for ZST in destroy_object() and always destroy?

pub struct AtomicArc<T> {
    object: AtomicPtr<T>,
    _marker: PhantomData<Option<Arc<T>>>,
}

unsafe impl<T: Send + Sync> Send for AtomicArc<T> {}
unsafe impl<T: Send + Sync> Sync for AtomicArc<T> {}

#[allow(dead_code)]
impl<T> AtomicArc<T> {
    pub fn new<U>(val: U) -> AtomicArc<T>
    where
        U: Into<Option<Arc<T>>>,
    {
        AtomicArc {
            object: AtomicPtr::new(into_raw(val)),
            _marker: PhantomData,
        }
    }

    pub fn into_inner(self) -> Option<Arc<T>> {
        let raw = self.object.load(Ordering::Relaxed);
        mem::forget(self);

        if raw.is_null() {
            None
        } else {
            unsafe { Some(Arc::from_raw(raw)) }
        }
    }

    pub fn get(&self) -> SharedArc<T> {
        let slot = unsafe { &*hazard::allocate_slot() };

        let mut object = self.object.load(Ordering::Relaxed);

        loop {
            if cfg!(any(target_arch = "x86", target_arch = "x86_64")) {
                // HACK(stjepang): On x86 architectures there are two different ways of executing a
                // `SeqCst` fence.
                //
                // 1. `atomic::fence(SeqCst)`, which compiles into a `mfence` instruction.
                // 2. `_.compare_and_swap(_, _, SeqCst)`, which compiles into a `lock cmpxchg`
                //    instruction.
                //
                // Both instructions have the effect of a full barrier, but benchmarks have shown
                // that the second one makes the algorithm faster in this particular case.
                let previous = slot.compare_and_swap(0, object as usize, Ordering::SeqCst);
                debug_assert_eq!(previous, 0);
            } else {
                slot.store(object as usize, Ordering::Relaxed);
                atomic::fence(Ordering::SeqCst);
            }

            let shared = SharedArc {
                object,
                slot,
                _marker: PhantomData,
            };

            let new = self.object.load(Ordering::Relaxed);
            if new == object {
                return shared;
            }
            object = new;

            // Deallocate the slot, potentially destroying the protected object.
            drop(shared);
        }
    }

    pub fn replace<U>(&self, val: U) -> SharedArc<T>
    where
        U: Into<Option<Arc<T>>>,
    {
        let old = self.object.swap(into_raw(val), Ordering::SeqCst);
        SharedArc {
            object: old,
            slot: ptr::null(),
            _marker: PhantomData,
        }
    }

    pub fn set<U>(&self, val: U)
    where
        U: Into<Option<Arc<T>>>,
    {
        self.replace(val.into());
    }

    #[allow(clippy::collapsible_if)]
    pub fn compare_and_set<U>(&self, current: &SharedArc<T>, new: U) -> Result<(), Option<Arc<T>>>
    where
        U: Into<Option<Arc<T>>>,
    {
        let new = into_raw(new);
        let old = current.object;

        if self.object.compare_and_swap(old, new, Ordering::SeqCst) == old {
            drop(SharedArc {
                object: old,
                slot: ptr::null(),
                _marker: PhantomData,
            });
            Ok(())
        } else {
            if new.is_null() {
                Err(None)
            } else {
                unsafe { Err(Some(Arc::from_raw(new))) }
            }
        }
    }
}

impl<T> Drop for AtomicArc<T> {
    fn drop(&mut self) {
        // 1) Either somebody is holding a reference to this element and we want to move
        //    responsibility of calling a drop(T) to them.
        // 2) Nobody is holding a reference to this element, therefore we are in charge of dropping
        //    an element.

        let obj = self.object.load(Ordering::Relaxed);

        if hazard::registry().destroy_object(obj as usize) {
            unsafe {
                drop(Arc::from_raw(obj));
            }
        }
    }
}

impl<T, U> From<U> for AtomicArc<T>
where
    U: Into<Option<Arc<T>>>,
{
    fn from(val: U) -> AtomicArc<T> {
        AtomicArc {
            object: AtomicPtr::new(into_raw(val)),
            _marker: PhantomData,
        }
    }
}

pub struct SharedArc<T> {
    object: *mut T,
    slot: *const AtomicUsize,
    _marker: PhantomData<Option<Arc<T>>>,
}

impl<T> SharedArc<T> {
    pub fn clone_inner(&self) -> Option<Arc<T>> {
        let arc = if self.object.is_null() {
            None
        } else {
            unsafe { Some(Arc::from_raw(self.object)) }
        };

        // Increment the reference count.
        mem::forget(arc.clone());

        arc
    }

    pub fn as_ref(&self) -> Option<&T> {
        unsafe { self.object.as_ref() }
    }

    // TODO: try_unwrap() and wait_unwrap()

    fn slot(&self) -> Option<&AtomicUsize> {
        unsafe { self.slot.as_ref() }
    }
}

impl<T> Drop for SharedArc<T> {
    #[inline]
    fn drop(&mut self) {
        // Set the slot back to zero. If it has been modified, that means we've been notified that
        // the object must be destroyed and it's our responsibility to do so.
        //
        // If the slot is not even allocatedt, this `SharedArc` was implicitly responsible for
        // destroying the object from the start.
        if let Some(slot) = self.slot() {
            if slot.swap(0, Ordering::Acquire) == self.object as usize {
                // Nothing to do! This is the common case.
                return;
            }
        }

        #[cold]
        fn destroy<T>(object: *const T) {
            if hazard::registry().destroy_object(object as usize) {
                unsafe {
                    drop(Arc::from_raw(object));
                }
            }
        }
        destroy(self.object)
    }
}

impl<T> Into<Option<Arc<T>>> for SharedArc<T> {
    fn into(self) -> Option<Arc<T>> {
        self.clone_inner()
    }
}

impl<'a, T> Into<Option<Arc<T>>> for &'a SharedArc<T> {
    fn into(self) -> Option<Arc<T>> {
        self.clone_inner()
    }
}

fn into_raw<T, U>(val: U) -> *mut T
where
    U: Into<Option<Arc<T>>>,
{
    match val.into() {
        None => ptr::null_mut(),
        Some(val) => Arc::into_raw(val) as *mut T,
    }
}
