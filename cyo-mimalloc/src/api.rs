//! Statistics, options, the version and process information.

use crate::MiMalloc;
use core::ffi::{c_long, c_void};
use core::fmt;
use cyo_mimalloc_sys::mi_option_t;

/// Tells mimalloc that the current thread is a worker of a thread pool.
///
/// mimalloc then does not move pages that other threads abandoned into this
/// thread. Call it on each worker thread of a thread pool of your own.
/// Calling it again has no further effect.
#[inline]
pub fn set_current_thread_in_threadpool() {
    // SAFETY: `mi_thread_set_in_threadpool` has no preconditions.
    unsafe { cyo_mimalloc_sys::mi_thread_set_in_threadpool() }
}

/// The time and memory use of the process, from [`MiMalloc::process_info`].
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessInfo {
    /// The wall-clock time in milliseconds since the process started.
    pub elapsed_msecs: usize,
    /// The CPU time in milliseconds that the process spent in user mode.
    pub user_msecs: usize,
    /// The CPU time in milliseconds that the process spent in the kernel.
    pub system_msecs: usize,
    /// The current resident set size in bytes.
    ///
    /// On Linux, mimalloc cannot read this value, and reports the memory it
    /// has committed instead.
    pub current_rss: usize,
    /// The largest resident set size in bytes that the process has had.
    pub peak_rss: usize,
    /// The number of bytes that mimalloc has committed.
    pub current_commit: usize,
    /// The largest number of bytes that mimalloc has had committed.
    pub peak_commit: usize,
    /// The number of page faults that the process has had. On Linux, this is
    /// the number of major faults.
    pub page_faults: usize,
}

impl MiMalloc {
    /// Returns the mimalloc version as `major * 10000 + minor * 100 + patch`,
    /// for example 30503 for v3.5.3.
    #[inline]
    pub fn version() -> i32 {
        // SAFETY: `mi_version` has no preconditions.
        unsafe { cyo_mimalloc_sys::mi_version() }
    }

    /// Returns memory that the current thread no longer uses.
    ///
    /// With `force`, mimalloc also purges unused memory immediately, instead
    /// of after `mi_option_purge_delay`.
    #[inline]
    pub fn collect(force: bool) {
        // SAFETY: `mi_collect` has no preconditions.
        unsafe { cyo_mimalloc_sys::mi_collect(force) }
    }

    /// Returns the usable size in bytes of the block at `ptr`.
    ///
    /// The usable size can be larger than the size that was requested.
    ///
    /// # Safety
    ///
    /// `ptr` must be a block that mimalloc allocated and that you have not
    /// freed.
    #[inline]
    pub unsafe fn usable_size(ptr: *const u8) -> usize {
        // SAFETY: the caller guarantees that `ptr` is a live mimalloc block.
        unsafe { cyo_mimalloc_sys::mi_usable_size(ptr as *const c_void) }
    }

    /// Returns the time and memory use of the process.
    pub fn process_info() -> ProcessInfo {
        let mut info = ProcessInfo::default();
        // SAFETY: every argument points to a field of `info`, which is valid
        // for writes.
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

    /// Writes the allocation statistics to `out` as JSON.
    ///
    /// # Errors
    ///
    /// Returns [`fmt::Error`] if mimalloc cannot produce the statistics, or if
    /// `out` returns an error.
    pub fn stats_json(out: &mut (impl fmt::Write + ?Sized)) -> fmt::Result {
        // SAFETY: with a null buffer, `mi_stats_get_json` returns a string
        // that mimalloc allocated and that the caller owns, or null.
        unsafe {
            crate::ffi::write_owned_c_string(
                out,
                cyo_mimalloc_sys::mi_stats_get_json(0, core::ptr::null_mut()),
            )
        }
    }

    /// Writes the allocation statistics to `out` as mimalloc's text table.
    ///
    /// # Errors
    ///
    /// Returns [`fmt::Error`] if `out` returns an error.
    pub fn stats_print(out: &mut dyn fmt::Write) -> fmt::Result {
        // SAFETY: `write_output` passes a callback and an argument that stay
        // valid for the call.
        crate::ffi::write_output(out, |out, arg| unsafe {
            cyo_mimalloc_sys::mi_stats_print_out(out, arg);
        })
    }

    /// Resets the allocation statistics to zero.
    #[inline]
    pub fn stats_reset() {
        // SAFETY: `mi_stats_reset` has no preconditions.
        unsafe { cyo_mimalloc_sys::mi_stats_reset() }
    }

    /// Writes the time and memory use of the process to `out` as mimalloc's
    /// text table.
    ///
    /// # Errors
    ///
    /// Returns [`fmt::Error`] if `out` returns an error.
    pub fn process_info_print(out: &mut dyn fmt::Write) -> fmt::Result {
        // SAFETY: `write_output` passes a callback and an argument that stay
        // valid for the call.
        crate::ffi::write_output(out, |out, arg| unsafe {
            cyo_mimalloc_sys::mi_process_info_print_out(out, arg);
        })
    }

    // ── Options ─────────────────────────────────────────────────────────────
    //
    // The crate documentation describes how options are set, and
    // `mi_option_t` describes each option and when mimalloc reads it.

    /// Returns whether an option is enabled, which means its value is not 0.
    #[inline]
    pub fn option_is_enabled(option: mi_option_t) -> bool {
        // SAFETY: `mi_option_is_enabled` has no preconditions.
        unsafe { cyo_mimalloc_sys::mi_option_is_enabled(option) }
    }

    /// Returns the current value of an option.
    ///
    /// For an option measured in KiB, the value is in KiB. To get it in bytes,
    /// call [`option_get_size`](Self::option_get_size).
    #[inline]
    pub fn option_get(option: mi_option_t) -> c_long {
        // SAFETY: `mi_option_get` has no preconditions.
        unsafe { cyo_mimalloc_sys::mi_option_get(option) }
    }

    /// Returns the current value in bytes of an option measured in KiB.
    #[inline]
    pub fn option_get_size(option: mi_option_t) -> usize {
        // SAFETY: `mi_option_get_size` has no preconditions.
        unsafe { cyo_mimalloc_sys::mi_option_get_size(option) }
    }

    /// Sets an option to `value`. For an option measured in KiB, `value` is in
    /// KiB.
    ///
    /// The value overrides the build-time default and the `MIMALLOC_*`
    /// environment variable. It only has an effect if mimalloc reads the option
    /// again afterwards. mimalloc reads some options only at startup, before
    /// `main` runs. [`mi_option_t`] marks them, and the crate documentation
    /// describes how to set them.
    ///
    /// ```rust
    /// use cyo_mimalloc::{MiMalloc, mi_option_t};
    ///
    /// // Return unused memory to the OS immediately.
    /// MiMalloc::option_set(mi_option_t::mi_option_purge_delay, 0);
    /// ```
    #[inline]
    pub fn option_set(option: mi_option_t, value: c_long) {
        // SAFETY: `mi_option_set` has no preconditions.
        unsafe { cyo_mimalloc_sys::mi_option_set(option, value) }
    }

    /// Sets an option to 1, as [`option_set`](Self::option_set) does.
    #[inline]
    pub fn option_enable(option: mi_option_t) {
        // SAFETY: `mi_option_enable` has no preconditions.
        unsafe { cyo_mimalloc_sys::mi_option_enable(option) }
    }

    /// Sets an option to 0, as [`option_set`](Self::option_set) does.
    #[inline]
    pub fn option_disable(option: mi_option_t) {
        // SAFETY: `mi_option_disable` has no preconditions.
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
