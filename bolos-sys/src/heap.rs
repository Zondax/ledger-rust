// Code taken from
// https://github.com/Zondax/embedded-alloc
// https://crates.io/crates/embedded-alloc/0.5.1

// LLHeap with PIC on RefCell
// This was required to PIC the RefCell on the Heap struct. Otherwise, the app will fail with segmentation fault.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::RefCell;
use core::ptr::{self, NonNull};

use critical_section::Mutex;
use linked_list_allocator::Heap as LLHeap;

use crate::pic::PIC;

/// A linked list first fit heap.
pub struct Heap {
    pub heap: Mutex<PIC<RefCell<LLHeap>>>,
}

impl Heap {
    /// Create a new UNINITIALIZED heap allocator
    ///
    /// You must initialize this heap using the
    /// [`init`](Self::init) method before using the allocator.
    pub const fn empty() -> Heap {
        Heap {
            heap: Mutex::new(PIC::new(RefCell::new(LLHeap::empty()))),
        }
    }

    /// Initializes the heap
    ///
    /// This function must be called BEFORE you run any code that makes use of the
    /// allocator.
    ///
    /// `start_addr` is the address where the heap will be located.
    ///
    /// `size` is the size of the heap in bytes.
    ///
    /// Note that:
    ///
    /// - The heap grows "upwards", towards larger addresses. Thus `start_addr` will
    ///   be the smallest address used.
    ///
    /// - The largest address used is `start_addr + size - 1`, so if `start_addr` is
    ///   `0x1000` and `size` is `0x30000` then the allocator won't use memory at
    ///   addresses `0x31000` and larger.
    ///
    /// # Safety
    ///
    /// Obey these or Bad Stuff will happen.
    ///
    /// - This function must be called exactly ONCE.
    /// - `size > 0`
    pub unsafe fn init(&self, start_addr: usize, size: usize) {
        // zlog("-- init 1 --\x00");
        critical_section::with(|cs| {
            self.heap
                .borrow(cs)
                .borrow_mut()
                .init(start_addr as *mut u8, size);
        });
    }

    /// Returns an estimate of the amount of bytes in use.
    pub fn used(&self) -> usize {
        // zlog("-- used 1 --\x00");
        critical_section::with(|cs| self.heap.borrow(cs).borrow_mut().used())
    }

    /// Returns an estimate of the amount of bytes available.
    pub fn free(&self) -> usize {
        // zlog("-- free 1 --\x00");
        critical_section::with(|cs| self.heap.borrow(cs).borrow_mut().free())
    }

    pub fn alloc(&self, layout: Layout) -> Option<NonNull<u8>> {
        // zlog("-- alloc 1 --\x00");
        let res = critical_section::with(|cs| {
            // zlog("-- alloc 2 --\x00");
            let heap = &self.heap;
            // zlog("-- alloc 3 --\x00");
            let heap2 = heap.borrow(cs);
            // zlog("-- alloc 4 --\x00");
            let mut heap3 = heap2.borrow_mut();
            // zlog("-- alloc 5 --\x00");
            let result = heap3.allocate_first_fit(layout);
            // zlog("-- alloc 6 --\x00");
            result.ok()
        });
        // zlog("-- alloc 7 --\x00");
        res
    }

    pub unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // zlog("-- dealloc 1 --\x00");
        critical_section::with(|cs| {
            self.heap
                .borrow(cs)
                .borrow_mut()
                .deallocate(NonNull::new_unchecked(ptr), layout)
        });
    }
}

unsafe impl GlobalAlloc for PIC<Heap> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // zlog("-- GlobalAlloc alloc 1 --\x00");
        self.get_ref().alloc(layout)
            .map_or(ptr::null_mut(), |allocation| allocation.as_ptr())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // zlog("-- GlobalAlloc dealloc 1 --\x00");
        self.get_ref().dealloc(ptr, layout);
    }
}

#[cfg(feature = "allocator_api")]
mod allocator_api {
    use super::*;
    use core::alloc::{AllocError, Allocator};

    unsafe impl Allocator for Heap {
        fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
            zlog("-- Allocator allocate 1 --\x00");
            match layout.size() {
                0 => Ok(NonNull::slice_from_raw_parts(layout.dangling(), 0)),
                size => self.alloc(layout).map_or(Err(AllocError), |allocation| {
                    Ok(NonNull::slice_from_raw_parts(allocation, size))
                }),
            }
        }

        unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
            zlog("-- Allocator deallocate 1 --\x00");
            if layout.size() != 0 {
                self.dealloc(ptr.as_ptr(), layout);
            }
        }
    }
}