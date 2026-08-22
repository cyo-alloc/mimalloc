//! Extended API: stats, options, version, process information.

use crate::MiMalloc;
use core::ffi::{c_void, CStr};

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

// ── Single impl block — no patch-on-patch ───────────────────────────────────

impl MiMalloc {
    /// mimalloc version as `major * 10000 + minor * 100 + patch`.
    #[inline]
    pub fn version() -> i32 {
        unsafe { rustfs_mimalloc_sys::mi_version() }
    }

    /// Force garbage collection.
    #[inline]
    pub fn collect(force: bool) {
        unsafe { rustfs_mimalloc_sys::mi_collect(force) }
    }

    /// Usable size of an allocated block (may be larger than requested).
    ///
    /// # Safety
    /// `ptr` must have been allocated by mimalloc.
    #[inline]
    pub unsafe fn usable_size(ptr: *const u8) -> usize {
        rustfs_mimalloc_sys::mi_usable_size(ptr as *const c_void)
    }

    /// Process memory information.
    pub fn process_info() -> ProcessInfo {
        let mut info = ProcessInfo::default();
        unsafe {
            rustfs_mimalloc_sys::mi_process_info(
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

    /// Allocation statistics as JSON. Returns empty string on failure.
    pub fn stats_json() -> String {
        unsafe {
            let ptr = rustfs_mimalloc_sys::mi_stats_get_json(0, core::ptr::null_mut());
            if ptr.is_null() {
                return String::new();
            }
            let cstr = core::ffi::CStr::from_ptr(ptr);
            let result = cstr.to_string_lossy().into_owned();
            rustfs_mimalloc_sys::mi_free(ptr as *mut c_void);
            result
        }
    }

    /// Allocation statistics in mimalloc's human-readable text format.
    pub fn stats_print() -> String {
        collect_mimalloc_output(|out, arg| unsafe {
            rustfs_mimalloc_sys::mi_stats_print_out(out, arg);
        })
    }

    /// Reset accumulated mimalloc allocation statistics.
    #[inline]
    pub fn stats_reset() {
        unsafe { rustfs_mimalloc_sys::mi_stats_reset() }
    }

    /// Process memory information in mimalloc's human-readable text format.
    pub fn process_info_print() -> String {
        collect_mimalloc_output(|out, arg| unsafe {
            rustfs_mimalloc_sys::mi_process_info_print_out(out, arg);
        })
    }

    // ── Options ─────────────────────────────────────────────────────────────

    /// Check if an option is enabled.
    #[inline]
    pub fn option_is_enabled(option: rustfs_mimalloc_sys::mi_option_t) -> bool {
        unsafe { rustfs_mimalloc_sys::mi_option_is_enabled(option) }
    }

    /// Get an option value.
    #[inline]
    pub fn option_get(option: rustfs_mimalloc_sys::mi_option_t) -> i64 {
        unsafe { rustfs_mimalloc_sys::mi_option_get(option) as i64 }
    }

    /// Get an option value as size (bytes).
    #[inline]
    pub fn option_get_size(option: rustfs_mimalloc_sys::mi_option_t) -> usize {
        unsafe { rustfs_mimalloc_sys::mi_option_get_size(option) }
    }

    /// Set an option value.
    ///
    /// ```rust
    /// use rustfs_mimalloc::MiMalloc;
    /// use rustfs_mimalloc_sys::mi_option_t;
    ///
    /// // Return memory to OS immediately
    /// MiMalloc::option_set(mi_option_t::mi_option_purge_delay, 0);
    /// ```
    #[inline]
    pub fn option_set(option: rustfs_mimalloc_sys::mi_option_t, value: i64) {
        unsafe { rustfs_mimalloc_sys::mi_option_set(option, value) }
    }

    /// Enable an option.
    #[inline]
    pub fn option_enable(option: rustfs_mimalloc_sys::mi_option_t) {
        unsafe { rustfs_mimalloc_sys::mi_option_enable(option) }
    }

    /// Disable an option.
    #[inline]
    pub fn option_disable(option: rustfs_mimalloc_sys::mi_option_t) {
        unsafe { rustfs_mimalloc_sys::mi_option_disable(option) }
    }
}

pub(crate) fn collect_mimalloc_output(
    write: impl FnOnce(Option<rustfs_mimalloc_sys::mi_output_fun>, *mut c_void),
) -> String {
    let mut output = Vec::new();
    write(
        Some(collect_mimalloc_output_callback),
        &mut output as *mut Vec<u8> as *mut c_void,
    );
    String::from_utf8_lossy(&output).into_owned()
}

unsafe extern "C" fn collect_mimalloc_output_callback(
    msg: *const rustfs_mimalloc_sys::c_char,
    arg: *mut c_void,
) {
    if msg.is_null() || arg.is_null() {
        return;
    }

    let output = unsafe { &mut *(arg as *mut Vec<u8>) };
    let msg = unsafe { CStr::from_ptr(msg) };
    output.extend_from_slice(msg.to_bytes());
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rustfs_mimalloc_sys::mi_option_t;

    #[test]
    fn version_is_v3() {
        assert!(MiMalloc::version() >= 30500, "expected >= V3.5.0");
    }

    #[test]
    fn stats_json_not_empty() {
        let json = MiMalloc::stats_json();
        assert!(!json.is_empty());
    }

    #[test]
    fn stats_print_not_empty() {
        let stats = MiMalloc::stats_print();
        assert!(!stats.is_empty());
    }

    #[test]
    fn stats_reset_smoke() {
        MiMalloc::stats_reset();
    }

    #[test]
    fn process_info_print_not_empty() {
        let info = MiMalloc::process_info_print();
        assert!(!info.is_empty());
    }

    #[test]
    fn option_roundtrip() {
        // Just verify no panic
        let _ = MiMalloc::option_get(mi_option_t::mi_option_purge_delay);
        let _ = MiMalloc::option_is_enabled(mi_option_t::mi_option_show_errors);
    }

    #[test]
    fn process_info_smoke() {
        let info = MiMalloc::process_info();
        let _ = info;
    }

    #[test]
    fn usable_size_at_least_requested() {
        unsafe {
            let ptr = rustfs_mimalloc_sys::mi_malloc(64);
            assert!(MiMalloc::usable_size(ptr as *const u8) >= 64);
            rustfs_mimalloc_sys::mi_free(ptr);
        }
    }
}
