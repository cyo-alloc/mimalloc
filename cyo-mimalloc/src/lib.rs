//! [mimalloc](https://github.com/microsoft/mimalloc) V3 as a Rust global
//! allocator, for Linux.
//!
//! ```rust
//! use cyo_mimalloc::MiMalloc;
//!
//! #[global_allocator]
//! static GLOBAL: MiMalloc = MiMalloc;
//! ```
//!
//! The crate is `no_std` and does not use `alloc`. Functions that produce text,
//! such as [`MiMalloc::stats_print`], write it to any [`core::fmt::Write`], so a
//! `String` works, and so does a fixed buffer or a logger that allocates
//! nothing.
//!
//! The global allocator only covers Rust allocations. C libraries linked into
//! the same program (through `-sys` crates) keep calling the system `malloc`.
//!
//! # Build configuration
//!
//! This crate has no Cargo features. Cargo merges features across the whole
//! dependency tree, so any library could switch on a mode that changes the
//! allocator for the entire program. Instead, mimalloc is configured through
//! environment variables of the build that produces the final binary (or
//! shared library). Put them in that project's `.cargo/config.toml`, or in the
//! environment of its Nix derivation or CI job; a dependency's own
//! `.cargo/config.toml` is not used. Changing any of them rebuilds mimalloc.
//!
//! | Variable | Values | Effect |
//! |---|---|---|
//! | `MI_SECURE` | 0 to 4 (default 0) | Secure mode: guard pages, encoded free lists, randomized allocation and double-free detection, more of them at higher levels, at some cost in speed. |
//! | `MI_DEBUG` | 0 to 3 (default 0) | Internal assertions (1), plus consistency checks (2), plus expensive checks (3). |
//! | `MI_NO_THP` | 0 or 1 (default 0) | 1 compiles out mimalloc's requests for transparent huge pages. To decide at runtime instead, use the `allow_thp` option. |
//! | `CYO_MIMALLOC_TLS_MODEL` | `initial-exec` (default), `local-dynamic`, `global-dynamic`, `local-exec` | How mimalloc finds its thread-local state. `initial-exec` is the fastest, but a shared library that is loaded with `dlopen` (such as a Python extension) needs `local-dynamic`. |
//! | `MI_DEFAULT_<NAME>` | see [Build-time defaults](#build-time-defaults) | The default value of a runtime option. |
//!
//! Invalid values stop the build with an error.
//!
//! ```toml
//! # .cargo/config.toml of the application
//! [env]
//! MI_SECURE = "4"
//! CYO_MIMALLOC_TLS_MODEL = "local-dynamic"
//! ```
//!
//! A library that depends on this crate should leave all of this to the
//! application, and should not declare a `#[global_allocator]` or call
//! [`MiMalloc::option_set`] either: those affect the whole program. For memory
//! of its own, it can use a [`heap::Heap`], optionally in an exclusive arena
//! from [`heap::reserve_os_memory`].
//!
//! # Options
//!
//! mimalloc has runtime options for things like how quickly unused memory is
//! returned to the OS and whether huge pages are used. [`mi_option_t`] lists
//! them all, with their defaults and when mimalloc reads each one. An option
//! gets its value from, in increasing order of precedence:
//!
//! 1. mimalloc's built-in default;
//! 2. a default chosen when your program is built;
//! 3. an environment variable when your program runs;
//! 4. a call from your code.
//!
//! So an application can ship its own defaults, and whoever runs it can still
//! tune them.
//!
//! ## Build-time defaults
//!
//! Like the rest of the [build configuration](#build-configuration), every
//! `MI_DEFAULT_<NAME>` variable in the build environment becomes the default
//! for the option `<name>`:
//!
//! ```toml
//! [env]
//! MI_DEFAULT_ALLOW_THP = "0"
//! MI_DEFAULT_ARENA_EAGER_COMMIT = "1"
//! MI_DEFAULT_RESERVE_OS_MEMORY = "1048576" # KiB, so 1 GiB
//! ```
//!
//! Only the options whose [`mi_option_t`] entry names a `MI_DEFAULT_*`
//! variable accept one, plus `MI_DEFAULT_PHYSICAL_MEMORY_IN_KIB`, the amount of
//! physical memory mimalloc assumes until it has detected the real amount.
//! Values are passed to the C compiler as they are, and sizes are in KiB. Any
//! other `MI_DEFAULT_*` variable is ignored.
//!
//! ## Environment variables
//!
//! When the program starts, mimalloc reads `MIMALLOC_<NAME>` for each option,
//! for example `MIMALLOC_PURGE_DELAY=0` or `MIMALLOC_ALLOW_THP=0`. The value
//! overrides the build-time default. Sizes accept a unit here:
//! `MIMALLOC_ARENA_RESERVE=4GiB`. Set `MIMALLOC_VERBOSE=1` to print every
//! option's value at startup.
//!
//! ## From code
//!
//! [`MiMalloc::option_set`] (with [`option_enable`](MiMalloc::option_enable)
//! and [`option_disable`](MiMalloc::option_disable)) overrides everything
//! else, but only from the moment it is called:
//!
//! ```rust
//! use cyo_mimalloc::{MiMalloc, mi_option_t};
//!
//! MiMalloc::option_set(mi_option_t::mi_option_purge_delay, 100);
//! MiMalloc::option_enable(mi_option_t::mi_option_purge_decommits);
//! ```
//!
//! mimalloc starts before any Rust code does, including `main`. Options it only
//! reads at startup are fixed by then, and setting them from code does nothing.
//! [`mi_option_t`] marks those "read at startup". Use a build-time default or
//! an environment variable for them, or the in-code equivalent where there is
//! one:
//!
//! | Option | In code |
//! |---|---|
//! | `reserve_os_memory` | [`heap::reserve_os_memory`] |
//! | `reserve_huge_os_pages`, `reserve_huge_os_pages_at` | [`heap::reserve_huge_os_pages_interleave`], [`heap::reserve_huge_os_pages_at`] |
//! | `allow_thp` = 0 | `prctl(PR_SET_THP_DISABLE, 1, 0, 0, 0)`, plus setting the option to 0 |
//! | `use_numa_nodes`, `max_vabits`, `pagemap_commit`, `max_errors`, `max_warnings` | none |
//!
//! Options read "per thread" apply to threads that start after the change.

#![no_std]

#[cfg(test)]
extern crate std;

mod api;
mod ffi;

pub mod heap;

pub use api::{ProcessInfo, set_current_thread_in_threadpool};
pub use cyo_mimalloc_sys::mi_option_t;

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
        unsafe { cyo_mimalloc_sys::mi_malloc_aligned(layout.size(), layout.align()) as *mut u8 }
    }

    #[inline(always)]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe { cyo_mimalloc_sys::mi_zalloc_aligned(layout.size(), layout.align()) as *mut u8 }
    }

    #[inline(always)]
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe { cyo_mimalloc_sys::mi_free(ptr as *mut c_void) };
    }

    #[inline(always)]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        unsafe {
            cyo_mimalloc_sys::mi_realloc_aligned(ptr as *mut c_void, new_size, layout.align())
                as *mut u8
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{GlobalAlloc, Layout};
    use std::boxed::Box;
    use std::vec::Vec;

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
