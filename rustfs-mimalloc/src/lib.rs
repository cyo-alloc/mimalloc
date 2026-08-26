//! High-performance [mimalloc](https://github.com/microsoft/mimalloc) V3 global allocator.
//!
//! ```rust
//! use rustfs_mimalloc::MiMalloc;
//!
//! #[global_allocator]
//! static GLOBAL: MiMalloc = MiMalloc;
//! ```

mod api;
mod ffi;

pub mod heap;

pub use api::{ProcessInfo, set_current_thread_in_threadpool};

use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;

/// The mimalloc global allocator.
///
/// Drop-in replacement for the system allocator. Always uses `mi_malloc_aligned`
/// internally to guarantee correct alignment for all layouts.
#[derive(Debug, Clone, Copy, Default)]
pub struct MiMalloc;

// ── GlobalAlloc: hot path — zero indirection ────────────────────────────────

unsafe impl GlobalAlloc for MiMalloc {
    #[inline(always)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { rustfs_mimalloc_sys::mi_malloc_aligned(layout.size(), layout.align()) as *mut u8 }
    }

    #[inline(always)]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe { rustfs_mimalloc_sys::mi_zalloc_aligned(layout.size(), layout.align()) as *mut u8 }
    }

    #[inline(always)]
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe { rustfs_mimalloc_sys::mi_free(ptr as *mut c_void) };
    }

    #[inline(always)]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        unsafe {
            rustfs_mimalloc_sys::mi_realloc_aligned(ptr as *mut c_void, new_size, layout.align())
                as *mut u8
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{GlobalAlloc, Layout};

    #[global_allocator]
    static GLOBAL: MiMalloc = MiMalloc;

    #[test]
    fn alloc_dealloc_roundtrip() {
        let layout = Layout::from_size_align(64, 8).unwrap();
        unsafe {
            let ptr = GLOBAL.alloc(layout);
            assert!(!ptr.is_null());
            GLOBAL.dealloc(ptr, layout);
        }
    }

    #[test]
    fn alloc_zeroed_is_zero() {
        let layout = Layout::from_size_align(256, 16).unwrap();
        unsafe {
            let ptr = GLOBAL.alloc_zeroed(layout);
            assert!(!ptr.is_null());
            assert!((0..256).all(|i| *ptr.add(i) == 0));
            GLOBAL.dealloc(ptr, layout);
        }
    }

    #[test]
    fn realloc_preserves_content() {
        let layout = Layout::from_size_align(64, 8).unwrap();
        unsafe {
            let ptr = GLOBAL.alloc(layout);
            core::ptr::write_bytes(ptr, 0xAB, 64);
            let new_ptr = GLOBAL.realloc(ptr, layout, 128);
            assert!(!new_ptr.is_null());
            assert!((0..64).all(|i| *new_ptr.add(i) == 0xAB));
            GLOBAL.dealloc(new_ptr, Layout::from_size_align(128, 8).unwrap());
        }
    }

    #[test]
    fn alignment_respected() {
        for align_pow in 0..=12 {
            let align = 1usize << align_pow;
            let layout = Layout::from_size_align(64, align).unwrap();
            unsafe {
                let ptr = GLOBAL.alloc(layout);
                assert!(!ptr.is_null(), "align={align}");
                assert_eq!(ptr as usize % align, 0, "align={align}");
                GLOBAL.dealloc(ptr, layout);
            }
        }
    }

    #[test]
    fn large_alignment_page_size() {
        // Regression: mimalloc_rust#87 — 4096 alignment crashed
        let layout = Layout::from_size_align(4096, 4096).unwrap();
        for _ in 0..100 {
            unsafe {
                let ptr = GLOBAL.alloc(layout);
                assert!(!ptr.is_null());
                assert_eq!(ptr as usize % 4096, 0);
                GLOBAL.dealloc(ptr, layout);
            }
        }
    }

    #[test]
    fn vec_push_smoke() {
        let v: Vec<i32> = (0..10_000).collect();
        assert_eq!(v.len(), 10_000);
        assert_eq!(v[9999], 9999);
    }

    #[test]
    fn box_smoke() {
        let b = Box::new(42u64);
        assert_eq!(*b, 42);
    }

    #[test]
    fn concurrent_alloc_free() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let n = 8;
        let barrier = Arc::new(Barrier::new(n));
        let handles: Vec<_> = (0..n)
            .map(|_| {
                let b = barrier.clone();
                thread::spawn(move || {
                    b.wait();
                    let layout = Layout::from_size_align(128, 16).unwrap();
                    for _ in 0..1000 {
                        unsafe {
                            let ptr = GLOBAL.alloc(layout);
                            assert!(!ptr.is_null());
                            core::ptr::write_bytes(ptr, 0xCD, 128);
                            GLOBAL.dealloc(ptr, layout);
                        }
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    }
}
