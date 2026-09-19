//! Stats, options, version, and process information APIs.

use crate::MiMalloc;
use core::ffi::{c_long, c_void};
use core::fmt;
use cyo_mimalloc_sys::mi_option_t;

/// Mark the current thread as part of a thread pool for mimalloc.
///
/// This is a safe wrapper around mimalloc V3's `mi_thread_set_in_threadpool`.
/// The upstream API takes no pointers, only updates the current thread's
/// mimalloc thread-local state, and is intended to be called by custom
/// thread-pool worker threads. Repeated calls keep the same threadpool marker.
#[inline]
pub fn set_current_thread_in_threadpool() {
    unsafe { cyo_mimalloc_sys::mi_thread_set_in_threadpool() }
}

/// Process memory information returned by [`MiMalloc::process_info`].
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessInfo {
    pub elapsed_msecs: usize,
    pub user_msecs: usize,
    pub system_msecs: usize,
    pub current_rss: usize,
    pub peak_rss: usize,
    pub current_commit: usize,
    pub peak_commit: usize,
    pub page_faults: usize,
}

impl MiMalloc {
    /// mimalloc version as `major * 10000 + minor * 100 + patch`.
    #[inline]
    pub fn version() -> i32 {
        unsafe { cyo_mimalloc_sys::mi_version() }
    }

    /// Force garbage collection.
    #[inline]
    pub fn collect(force: bool) {
        unsafe { cyo_mimalloc_sys::mi_collect(force) }
    }

    /// Usable size of an allocated block (may be larger than requested).
    ///
    /// # Safety
    /// `ptr` must have been allocated by mimalloc.
    #[inline]
    pub unsafe fn usable_size(ptr: *const u8) -> usize {
        unsafe { cyo_mimalloc_sys::mi_usable_size(ptr as *const c_void) }
    }

    /// Process memory information.
    pub fn process_info() -> ProcessInfo {
        let mut info = ProcessInfo::default();
        unsafe {
            cyo_mimalloc_sys::mi_process_info(
                &mut info.elapsed_msecs,
                &mut info.user_msecs,
                &mut info.system_msecs,
                &mut info.current_rss,
                &mut info.peak_rss,
                &mut info.current_commit,
                &mut info.peak_commit,
                &mut info.page_faults,
            );
        }
        info
    }

    // ── Stats ───────────────────────────────────────────────────────────────

    /// Write the allocation statistics to `out` as JSON.
    ///
    /// Fails if mimalloc cannot produce the statistics or `out` fails.
    pub fn stats_json(out: &mut (impl fmt::Write + ?Sized)) -> fmt::Result {
        unsafe {
            crate::ffi::write_owned_c_string(
                out,
                cyo_mimalloc_sys::mi_stats_get_json(0, core::ptr::null_mut()),
            )
        }
    }

    /// Write the allocation statistics to `out` in mimalloc's human-readable
    /// text format.
    pub fn stats_print(out: &mut dyn fmt::Write) -> fmt::Result {
        crate::ffi::write_output(out, |out, arg| unsafe {
            cyo_mimalloc_sys::mi_stats_print_out(out, arg);
        })
    }

    /// Reset accumulated mimalloc allocation statistics.
    #[inline]
    pub fn stats_reset() {
        unsafe { cyo_mimalloc_sys::mi_stats_reset() }
    }

    /// Write process memory information to `out` in mimalloc's human-readable
    /// text format.
    pub fn process_info_print(out: &mut dyn fmt::Write) -> fmt::Result {
        crate::ffi::write_output(out, |out, arg| unsafe {
            cyo_mimalloc_sys::mi_process_info_print_out(out, arg);
        })
    }

    // ── Options ─────────────────────────────────────────────────────────────
    //
    // See the crate documentation for how options are set, and `mi_option_t`
    // for what each one does and when mimalloc reads it.

    /// Check if an option is enabled (its value is non-zero).
    #[inline]
    pub fn option_is_enabled(option: mi_option_t) -> bool {
        unsafe { cyo_mimalloc_sys::mi_option_is_enabled(option) }
    }

    /// Get an option's current value. Options measured in KiB are returned in
    /// KiB; use [`option_get_size`](Self::option_get_size) for bytes.
    #[inline]
    pub fn option_get(option: mi_option_t) -> c_long {
        unsafe { cyo_mimalloc_sys::mi_option_get(option) }
    }

    /// Get an option's current value in bytes, for options measured in KiB.
    #[inline]
    pub fn option_get_size(option: mi_option_t) -> usize {
        unsafe { cyo_mimalloc_sys::mi_option_get_size(option) }
    }

    /// Set an option, overriding both the build-time default and the
    /// `MIMALLOC_*` environment variable. Options measured in KiB take KiB.
    ///
    /// This only has an effect if mimalloc reads the option again afterwards.
    /// Options read at startup (see [`mi_option_t`]) are already fixed by the
    /// time `main` runs; see the crate documentation for how to set those.
    ///
    /// ```rust
    /// use cyo_mimalloc::{MiMalloc, mi_option_t};
    ///
    /// // Return memory to OS immediately
    /// MiMalloc::option_set(mi_option_t::mi_option_purge_delay, 0);
    /// ```
    #[inline]
    pub fn option_set(option: mi_option_t, value: c_long) {
        unsafe { cyo_mimalloc_sys::mi_option_set(option, value) }
    }

    /// Set an option to 1. The same caveats as [`option_set`](Self::option_set) apply.
    #[inline]
    pub fn option_enable(option: mi_option_t) {
        unsafe { cyo_mimalloc_sys::mi_option_enable(option) }
    }

    /// Set an option to 0. The same caveats as [`option_set`](Self::option_set) apply.
    #[inline]
    pub fn option_disable(option: mi_option_t) {
        unsafe { cyo_mimalloc_sys::mi_option_disable(option) }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::String;

    #[test]
    fn version_is_v3() {
        assert!(MiMalloc::version() >= 30502, "expected >= V3.5.2");
    }

    #[test]
    fn stats_json_not_empty() {
        let mut json = String::new();
        MiMalloc::stats_json(&mut json).unwrap();
        assert!(json.starts_with('{'));
    }

    #[test]
    fn stats_print_not_empty() {
        let mut stats = String::new();
        MiMalloc::stats_print(&mut stats).unwrap();
        assert!(!stats.is_empty());
    }

    #[test]
    fn stats_print_propagates_writer_errors() {
        struct Failing;
        impl fmt::Write for Failing {
            fn write_str(&mut self, _: &str) -> fmt::Result {
                Err(fmt::Error)
            }
        }
        assert!(MiMalloc::stats_print(&mut Failing).is_err());
        assert!(MiMalloc::stats_json(&mut Failing).is_err());
    }

    #[test]
    fn stats_reset_smoke() {
        MiMalloc::stats_reset();
    }

    #[test]
    fn process_info_print_not_empty() {
        let mut info = String::new();
        MiMalloc::process_info_print(&mut info).unwrap();
        assert!(!info.is_empty());
    }

    #[test]
    fn option_roundtrip() {
        let option = mi_option_t::mi_option_purge_delay;
        let original = MiMalloc::option_get(option);
        MiMalloc::option_set(option, original + 7);
        assert_eq!(MiMalloc::option_get(option), original + 7);
        MiMalloc::option_set(option, original);
        assert_eq!(MiMalloc::option_get(option), original);

        let _ = MiMalloc::option_is_enabled(mi_option_t::mi_option_show_errors);
    }

    #[test]
    fn process_info_smoke() {
        let info = MiMalloc::process_info();
        let _ = info;
    }

    #[test]
    fn set_current_thread_in_threadpool_smoke() {
        set_current_thread_in_threadpool();
        set_current_thread_in_threadpool();
    }

    #[test]
    fn usable_size_at_least_requested() {
        unsafe {
            let ptr = cyo_mimalloc_sys::mi_malloc(64);
            assert!(MiMalloc::usable_size(ptr as *const u8) >= 64);
            cyo_mimalloc_sys::mi_free(ptr);
        }
    }
}
