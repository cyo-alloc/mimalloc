//! [mimalloc](https://github.com/microsoft/mimalloc) V3 as a Rust global
//! allocator.
//!
//! To use mimalloc for every Rust allocation, declare it as the global
//! allocator:
//!
//! ```rust
//! use cyo_mimalloc::MiMalloc;
//!
//! #[global_allocator]
//! static GLOBAL: MiMalloc = MiMalloc;
//! ```
//!
//! The crate is `no_std` and does not use `alloc`. Functions that produce text,
//! such as [`MiMalloc::stats_print`], write it to any [`core::fmt::Write`]. A
//! `String` works, and so do a fixed buffer and a logger that allocates
//! nothing.
//!
//! The global allocator only covers Rust allocations. A C library linked into
//! the same program, for example through a `-sys` crate, still calls the
//! system `malloc`.
//!
//! # Build configuration
//!
//! This crate has no Cargo features. Cargo merges features across the whole
//! dependency tree, so any library could switch on a mode that changes the
//! allocator for the entire program. Instead, you configure mimalloc through
//! environment variables of the build that produces the final binary or shared
//! library. Put them in that project's `.cargo/config.toml`, or in the
//! environment of its Nix derivation or CI job. Cargo does not read the
//! `.cargo/config.toml` of a dependency. Changing any of the variables rebuilds
//! mimalloc.
//!
//! | Variable | Values | Effect |
//! |---|---|---|
//! | `MI_SECURE` | 0 to 4 (default 0) | Secure mode. Higher levels add more protections, such as guard pages, encoded free lists, randomized allocation and double-free detection. Each protection costs some speed. |
//! | `MI_DEBUG` | 0 to 3 (default 0) | Internal assertions (1), plus consistency checks (2), plus expensive checks (3). |
//! | `MI_NO_THP` | 0 or 1 (default 0) | 1 compiles out mimalloc's requests for transparent huge pages. To decide at runtime instead, use the `allow_thp` option. |
//! | `CYO_MIMALLOC_TLS_MODEL` | `initial-exec` (default), `local-dynamic`, `global-dynamic`, `local-exec` | How mimalloc finds its thread-local state. `initial-exec` is the fastest. A shared library that is loaded with `dlopen`, such as a Python extension, needs `local-dynamic`. A build for MSVC checks the value but does not use it. |
//! | `MI_DEFAULT_<NAME>` | see [Build-time defaults](#build-time-defaults) | The default value of a runtime option. |
//!
//! An invalid value stops the build with an error.
//!
//! On MSVC, the build compiles mimalloc with the C runtime that the rest of the
//! build uses. That is `/MT` under `-C target-feature=+crt-static`, and `/MD`
//! otherwise. A statically linked program therefore needs neither
//! `vcruntime140.dll` nor `ucrtbase.dll`. No build variable of this crate
//! selects the runtime, because mixing the two runtimes in one binary is a link
//! error.
//!
//! ```toml
//! # .cargo/config.toml of the application
//! [env]
//! MI_SECURE = "4"
//! CYO_MIMALLOC_TLS_MODEL = "local-dynamic"
//! ```
//!
//! A library that depends on this crate should leave this configuration to the
//! application. It should also not declare a `#[global_allocator]` or call
//! [`MiMalloc::option_set`], because both affect the whole program. For memory
//! of its own, a library can use a [`heap::Heap`], optionally in an exclusive
//! arena from [`heap::reserve_os_memory`].
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
//! An application can therefore ship its own defaults, and whoever runs it can
//! still change them.
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
//! variable accept one. The build also accepts
//! `MI_DEFAULT_PHYSICAL_MEMORY_IN_KIB`, which sets the amount of physical memory
//! that mimalloc assumes until it has detected the real amount. The build passes
//! each value to the C compiler unchanged, and sizes are in KiB. The build
//! ignores any other `MI_DEFAULT_*` variable, with a warning.
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
//! [`MiMalloc::option_set`], [`option_enable`](MiMalloc::option_enable) and
//! [`option_disable`](MiMalloc::option_disable) override every other source,
//! from the moment you call them:
//!
//! ```rust
//! use cyo_mimalloc::{MiMalloc, mi_option_t};
//!
//! MiMalloc::option_set(mi_option_t::mi_option_purge_delay, 100);
//! MiMalloc::option_enable(mi_option_t::mi_option_purge_decommits);
//! ```
//!
//! mimalloc starts before any Rust code runs, including `main`. The options that
//! it only reads at startup are fixed by then, and setting them from code does
//! nothing. [`mi_option_t`] marks them "read at startup". Set them with a
//! build-time default or an environment variable, or call the function that
//! does the same at run time, where there is one:
//!
//! | Option | In code |
//! |---|---|
//! | `reserve_os_memory` | [`heap::reserve_os_memory`] |
//! | `reserve_huge_os_pages`, `reserve_huge_os_pages_at` | [`heap::reserve_huge_os_pages_interleave`], [`heap::reserve_huge_os_pages_at`] |
//! | `allow_thp` = 0 | `prctl(PR_SET_THP_DISABLE, 1, 0, 0, 0)`, plus setting the option to 0 |
//! | `use_numa_nodes`, `max_vabits`, `pagemap_commit`, `max_errors`, `max_warnings` | none |
//!
//! An option that mimalloc reads "per thread" applies to the threads that
//! start after you change it.

#![no_std]
#![warn(missing_docs)]
#![warn(clippy::undocumented_unsafe_blocks)]
#![warn(clippy::missing_errors_doc, clippy::missing_safety_doc)]
#![cfg_attr(test, allow(clippy::undocumented_unsafe_blocks))]

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
/// Every allocation goes through an aligned mimalloc function, such as
/// `mi_malloc_aligned`, so the block meets the alignment of its [`Layout`].
#[derive(Debug, Clone, Copy, Default)]
pub struct MiMalloc;

// ── GlobalAlloc ─────────────────────────────────────────────────────────────

// SAFETY: each method returns a block from mimalloc that meets the size and
// alignment of `layout`, or null. mimalloc frees and resizes only blocks that
// it allocated, and `GlobalAlloc` passes only those.
unsafe impl GlobalAlloc for MiMalloc {
    #[inline(always)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: `mi_malloc_aligned` has no preconditions. `Layout`
        // guarantees that the alignment is a power of two.
        unsafe { cyo_mimalloc_sys::mi_malloc_aligned(layout.size(), layout.align()) as *mut u8 }
    }

    #[inline(always)]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: as in `alloc`.
        unsafe { cyo_mimalloc_sys::mi_zalloc_aligned(layout.size(), layout.align()) as *mut u8 }
    }

    #[inline(always)]
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        // SAFETY: the caller of `dealloc` guarantees that `ptr` came from
        // this allocator and is still allocated.
        unsafe { cyo_mimalloc_sys::mi_free(ptr as *mut c_void) };
    }

    #[inline(always)]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller of `realloc` guarantees that `ptr` came from
        // this allocator with `layout` and is still allocated.
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
        // Page-sized alignment has crashed allocators that skip the aligned
        // mimalloc functions for alignments they assume are already met.
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
