//! Low-level FFI bindings to [mimalloc](https://github.com/microsoft/mimalloc) V3, for Linux.
//!
//! For a safe wrapper, use the `cyo-mimalloc` crate. [`mi_option_t`] documents
//! every runtime option and how to set it.

#![no_std]
#![allow(non_camel_case_types)]

// ── Type aliases ────────────────────────────────────────────────────────────

pub use core::ffi::{c_char, c_int, c_long, c_void};

pub type size_t = usize;

// ── Constants ──────────────────────────────────────────────────────────────

/// Maximum user data bytes stored inline with a sampled profiling allocation.
pub const MI_PROFILE_SAMPLE_DATA_MAX_SIZE: size_t = 1024;

// ── Opaque types ────────────────────────────────────────────────────────────

#[repr(C)]
pub struct mi_heap_t {
    _opaque: [u8; 0],
}

/// Subprocess identifier. Opaque handle — do not access fields directly.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mi_subproc_id_t {
    _id: *mut c_void,
}

/// Arena identifier. Opaque handle.
pub type mi_arena_id_t = *mut c_void;

// ── Option enum ─────────────────────────────────────────────────────────────
//
// Kept in sync with `mi_option_e` in `mimalloc.h`.

/// A mimalloc runtime option.
///
/// Every option can be set in three places. Later ones take precedence:
///
/// 1. At build time, through a `MI_DEFAULT_*` variable in the build environment,
///    for the options that list one below.
/// 2. In the environment of the running process, as `MIMALLOC_<NAME>`, where
///    `<NAME>` is the variant name without the `mi_option_` prefix, in upper
///    case. mimalloc reads these once, when the process starts.
/// 3. From code, with `mi_option_set` (or `MiMalloc::option_set` in
///    `cyo-mimalloc`).
///
/// Setting an option from code only matters if mimalloc reads it again
/// afterwards. Each variant says when it is read:
///
/// - **on use**: every time mimalloc needs it, so it can be changed at any
///   time.
/// - **per thread**: when a thread first allocates, so it applies to threads
///   that start afterwards.
/// - **at startup**: once, before `main`. Setting it from code has no effect;
///   use the environment variable or the build-time default instead.
/// - **at exit**: when the process ends.
///
/// Options measured in KiB accept a `K`, `M`, `G` or `T` suffix in the
/// environment (`MIMALLOC_ARENA_RESERVE=4GiB`); through `mi_option_set` and
/// `MI_DEFAULT_*` they are plain KiB. `mi_option_get_size` returns them in bytes.
/// Boolean options accept `1`/`0`, `true`/`false`, `on`/`off` and `yes`/`no`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum mi_option_t {
    /// Print error messages to stderr. Default 0, or 1 when built
    /// with `MI_DEBUG`. Read on use.
    mi_option_show_errors = 0,
    /// Print statistics to stderr when the process exits. Default 0. Read at
    /// exit.
    mi_option_show_stats = 1,
    /// Print verbose messages, including the option values at startup and
    /// statistics at exit. Default 0. Build-time default:
    /// `MI_DEFAULT_VERBOSE`. Read on use.
    mi_option_verbose = 2,
    /// Commit a whole arena when it is created instead of on demand: 0 = no,
    /// 1 = yes, 2 = only on systems that overcommit, which includes Linux.
    /// Committing does not raise the resident set. Default 2. Build-time
    /// default: `MI_DEFAULT_ARENA_EAGER_COMMIT`. Read on use, when an arena is
    /// created.
    mi_option_arena_eager_commit = 4,
    /// How to give unused memory back to the OS: 1 = decommit
    /// (`MADV_DONTNEED`, lowers the resident set immediately), 0 = reset
    /// (`MADV_FREE`, the kernel reclaims it when it needs to). Default 1. Read
    /// on use.
    mi_option_purge_decommits = 5,
    /// Use explicit large OS pages (2 MiB, `MAP_HUGETLB`) when available. The
    /// kernel only has these if `vm.nr_hugepages` is set, and they cannot be
    /// purged. Default 0. Build-time default:
    /// `MI_DEFAULT_ALLOW_LARGE_OS_PAGES`. Read on use.
    mi_option_allow_large_os_pages = 6,
    /// Number of 1 GiB huge pages to reserve at startup, spread over the NUMA
    /// nodes. Default 0. Build-time default:
    /// `MI_DEFAULT_RESERVE_HUGE_OS_PAGES`. Read at startup; from code, call
    /// `mi_reserve_huge_os_pages_interleave` instead.
    mi_option_reserve_huge_os_pages = 7,
    /// Reserve all huge pages from `mi_option_reserve_huge_os_pages` on this
    /// NUMA node instead of spreading them. Default -1 (spread). Read at
    /// startup; from code, call `mi_reserve_huge_os_pages_at` instead.
    mi_option_reserve_huge_os_pages_at = 8,
    /// Memory to reserve as an arena at startup, in KiB. Default 0. Build-time
    /// default: `MI_DEFAULT_RESERVE_OS_MEMORY`. Read at startup; from code,
    /// call `mi_reserve_os_memory_ex` instead.
    mi_option_reserve_os_memory = 9,
    /// Delay in milliseconds before unused memory is purged (see
    /// `mi_option_purge_decommits`). 0 purges immediately, -1 never purges.
    /// Default 1000. Read on use.
    mi_option_purge_delay = 15,
    /// Use at most this many NUMA nodes; 0 detects them. Setting 1 can help in
    /// virtual machines that report NUMA incorrectly. Default 0. Read at
    /// startup, and has no build-time default: set `MIMALLOC_USE_NUMA_NODES`.
    mi_option_use_numa_nodes = 16,
    /// Never allocate memory from the OS, only from arenas that were reserved
    /// up front. Default 0. Read on use.
    mi_option_disallow_os_alloc = 17,
    /// macOS memory tag. No effect on Linux.
    mi_option_os_tag = 18,
    /// Maximum number of error messages to print. Default 32. Read at startup.
    mi_option_max_errors = 19,
    /// Maximum number of warning messages to print. Default 32. Read at
    /// startup.
    mi_option_max_warnings = 20,
    /// Release all OS memory when the process exits, even if other code may
    /// still use it afterwards. Default 0. Read at exit.
    mi_option_destroy_on_exit = 22,
    /// Size of each arena mimalloc reserves from the OS, in KiB. Default
    /// 1 GiB. Build-time default: `MI_DEFAULT_ARENA_RESERVE`. Read on use, when
    /// an arena is created.
    mi_option_arena_reserve = 23,
    /// Multiplier on `mi_option_purge_delay` for purging arena memory. Default
    /// 4. Read on use.
    mi_option_arena_purge_mult = 24,
    /// Never allocate from arenas, except arenas requested explicitly by id.
    /// Default 0. Build-time default: `MI_DEFAULT_DISALLOW_ARENA_ALLOC`. Read
    /// on use.
    mi_option_disallow_arena_alloc = 26,
    /// Windows only. No effect on Linux.
    mi_option_retry_on_oom = 27,
    /// Guarded allocation: minimum object size. Only has an effect in builds
    /// with `MI_GUARDED`, which this crate does not enable.
    mi_option_guarded_min = 29,
    /// Guarded allocation: maximum object size. Only has an effect in builds
    /// with `MI_GUARDED`, which this crate does not enable.
    mi_option_guarded_max = 30,
    /// Guarded allocation: place blocks exactly against the guard page. Only
    /// has an effect in builds with `MI_GUARDED`, which this crate does not
    /// enable.
    mi_option_guarded_precise = 31,
    /// Guarded allocation: guard 1 in N allocations. Only has an effect in
    /// builds with `MI_GUARDED`, which this crate does not enable.
    mi_option_guarded_sample_rate = 32,
    /// Guarded allocation: random seed for sampling. Only has an effect in
    /// builds with `MI_GUARDED`, which this crate does not enable.
    mi_option_guarded_sample_seed = 33,
    /// Collect a thread's heap every N allocations that take the slow path.
    /// Default 10000. Read on use.
    mi_option_generic_collect = 34,
    /// Reclaim abandoned pages when a block in them is freed: -1 = never,
    /// 0 = only into the thread that owned them, 1 = into any thread. Default
    /// 0. Read per thread and on use.
    mi_option_page_reclaim_on_free = 35,
    /// Number of empty pages each thread keeps in its free queues; -1 disables
    /// abandoning pages. Default 2. Read per thread.
    mi_option_page_full_retain = 36,
    /// Maximum number of pages to search for the best fit. Default 4. Read on
    /// use.
    mi_option_page_max_candidates = 37,
    /// Maximum usable virtual address bits; 0 detects them. Default 0. Read at
    /// startup.
    mi_option_max_vabits = 38,
    /// Commit the whole page map at startup instead of on demand. Default 0.
    /// Build-time default: `MI_DEFAULT_PAGEMAP_COMMIT`. Read at startup.
    mi_option_pagemap_commit = 39,
    /// Commit pages on demand: 0 = no, 1 = yes, 2 = only on systems that do not
    /// overcommit. Default 0. Read on use.
    mi_option_page_commit_on_demand = 40,
    /// Do not reclaim a page into its original thread if that thread already
    /// owns this many pages of the same size class; -1 = no limit. Default -1.
    /// Build-time default: `MI_DEFAULT_PAGE_MAX_RECLAIM`. Read on use.
    mi_option_page_max_reclaim = 41,
    /// Do not reclaim a page across threads if the thread already owns this
    /// many pages of the same size class. Default 32. Build-time default:
    /// `MI_DEFAULT_PAGE_CROSS_THREAD_MAX_RECLAIM`. Read on use.
    mi_option_page_cross_thread_max_reclaim = 42,
    /// Transparent huge pages: 0 = disable THP for the whole process,
    /// 1 = allow, 2 = allow and raise `mi_option_minimal_purge_size` to 2 MiB
    /// so purging does not split huge pages. Default 2. Build-time default:
    /// `MI_DEFAULT_ALLOW_THP`. Disabling THP for the process happens at
    /// startup; whether mimalloc asks for huge pages on new memory is read on
    /// use.
    mi_option_allow_thp = 43,
    /// Smallest amount of memory to purge at once, in KiB; 0 picks 64 KiB, or
    /// 2 MiB when THP is available and `mi_option_allow_thp` is 2. Default 0.
    /// Read on use.
    mi_option_minimal_purge_size = 44,
    /// Largest object allocated from an arena, in KiB; larger ones come
    /// straight from the OS. Default 2 GiB. Build-time default:
    /// `MI_DEFAULT_ARENA_MAX_OBJECT_SIZE`. Read on use.
    mi_option_arena_max_object_size = 45,
    /// Tie a new arena to the NUMA node of the thread that created it.
    /// Default 0. Read on use.
    mi_option_arena_is_numa_local = 46,
    /// Merge a thread heap's statistics into its parent heap on every collect.
    /// Default 1. Build-time default: `MI_DEFAULT_COLLECT_MERGES_STATS`. Read
    /// on use.
    mi_option_collect_merges_stats = 47,
}

// ── Heap area (for visiting blocks) ─────────────────────────────────────────

#[repr(C)]
pub struct mi_heap_area_t {
    pub blocks: *mut c_void,
    pub reserved: size_t,
    pub committed: size_t,
    pub used: size_t,
    pub block_size: size_t,
    pub full_block_size: size_t,
    pub reserved1: *mut c_void,
}

// ── Callback types ──────────────────────────────────────────────────────────

pub type mi_output_fun = unsafe extern "C" fn(msg: *const c_char, arg: *mut c_void);
pub type mi_error_fun = unsafe extern "C" fn(err: c_int, arg: *mut c_void);
pub type mi_deferred_free_fun = unsafe extern "C" fn(force: bool, heartbeat: u64, arg: *mut c_void);
pub type mi_profiler_on_alloc_fun = unsafe extern "C" fn(
    profiler: *mut mi_profiler_t,
    profiler_data: *mut mi_profiler_sample_data_t,
    ptr: *mut c_void,
    requested_size: size_t,
    bytes_sample_rate: size_t,
    bytes_since_last_sample: u64,
    heap: *const mi_heap_t,
) -> size_t;
pub type mi_profiler_on_realloc_inplace_fun = unsafe extern "C" fn(
    profiler: *mut mi_profiler_t,
    profiler_data: *mut mi_profiler_sample_data_t,
    ptr: *mut c_void,
    old_size: size_t,
    heap: *const mi_heap_t,
) -> size_t;
pub type mi_profiler_on_free_fun = unsafe extern "C" fn(
    profiler: *mut mi_profiler_t,
    profiler_data: *mut mi_profiler_sample_data_t,
    ptr: *mut c_void,
    heap: *const mi_heap_t,
);
pub type mi_block_visit_fun = unsafe extern "C" fn(
    heap: *const mi_heap_t,
    area: *const mi_heap_area_t,
    block: *mut c_void,
    block_size: size_t,
    arg: *mut c_void,
) -> bool;
pub type mi_heap_visit_fun = unsafe extern "C" fn(heap: *mut mi_heap_t, arg: *mut c_void) -> bool;

// ── Profiling ───────────────────────────────────────────────────────────────

#[repr(C)]
pub struct mi_profiler_sample_data_t {
    pub user_data_size: size_t,
    pub user_data: [*mut c_void; 1],
}

/// Experimental mimalloc profiling hook table.
#[repr(C)]
pub struct mi_profiler_t {
    pub reserved: *mut c_void,
    pub sample_data_size: size_t,
    pub initial_sample_rate: size_t,
    pub on_alloc: Option<mi_profiler_on_alloc_fun>,
    pub on_free: Option<mi_profiler_on_free_fun>,
    pub on_realloc_inplace: Option<mi_profiler_on_realloc_inplace_fun>,
}

// ── Standard malloc interface ───────────────────────────────────────────────

unsafe extern "C" {
    pub fn mi_malloc(size: size_t) -> *mut c_void;
    pub fn mi_calloc(count: size_t, size: size_t) -> *mut c_void;
    pub fn mi_realloc(p: *mut c_void, newsize: size_t) -> *mut c_void;
    pub fn mi_free(p: *mut c_void);
    pub fn mi_strdup(s: *const c_char) -> *mut c_char;
}

// ── Extended allocation ─────────────────────────────────────────────────────

unsafe extern "C" {
    pub fn mi_malloc_small(size: size_t) -> *mut c_void;
    pub fn mi_zalloc_small(size: size_t) -> *mut c_void;
    pub fn mi_wmalloc_small(wsize: size_t) -> *mut c_void;
    pub fn mi_wzalloc_small(wsize: size_t) -> *mut c_void;
    pub fn mi_zalloc(size: size_t) -> *mut c_void;
    pub fn mi_mallocn(count: size_t, size: size_t) -> *mut c_void;
    pub fn mi_reallocn(p: *mut c_void, count: size_t, size: size_t) -> *mut c_void;
    pub fn mi_usable_size(p: *const c_void) -> size_t;
    pub fn mi_good_size(size: size_t) -> size_t;
    pub fn mi_free_size(p: *mut c_void, size: size_t);
    pub fn mi_free_small(p: *mut c_void);
    pub fn mi_free_small_nonnull(p: *mut c_void);
}

// ── Aligned allocation ──────────────────────────────────────────────────────

unsafe extern "C" {
    pub fn mi_malloc_aligned(size: size_t, alignment: size_t) -> *mut c_void;
    pub fn mi_zalloc_aligned(size: size_t, alignment: size_t) -> *mut c_void;
    pub fn mi_calloc_aligned(count: size_t, size: size_t, alignment: size_t) -> *mut c_void;
    pub fn mi_realloc_aligned(p: *mut c_void, newsize: size_t, alignment: size_t) -> *mut c_void;
}

// ── Process & thread lifecycle ──────────────────────────────────────────────

unsafe extern "C" {
    pub fn mi_collect(force: bool);
    pub fn mi_thread_set_in_threadpool();
    pub fn mi_version() -> c_int;
    pub fn mi_process_info_print_out(out: Option<mi_output_fun>, arg: *mut c_void);
    pub fn mi_process_info(
        elapsed_msecs: *mut size_t,
        user_msecs: *mut size_t,
        system_msecs: *mut size_t,
        current_rss: *mut size_t,
        peak_rss: *mut size_t,
        current_commit: *mut size_t,
        peak_commit: *mut size_t,
        page_faults: *mut size_t,
    );
}

// ── Heaps ───────────────────────────────────────────────────────────────────

unsafe extern "C" {
    pub fn mi_heap_new() -> *mut mi_heap_t;
    pub fn mi_heap_delete(heap: *mut mi_heap_t);
    pub fn mi_heap_destroy(heap: *mut mi_heap_t);
    pub fn mi_heap_collect(heap: *mut mi_heap_t, force: bool);
    pub fn mi_heap_main() -> *mut mi_heap_t;
    pub fn mi_heap_of(p: *const c_void) -> *mut mi_heap_t;
    pub fn mi_heap_contains(heap: *const mi_heap_t, p: *const c_void) -> bool;

    pub fn mi_heap_malloc(heap: *mut mi_heap_t, size: size_t) -> *mut c_void;
    pub fn mi_heap_zalloc(heap: *mut mi_heap_t, size: size_t) -> *mut c_void;
    pub fn mi_heap_calloc(heap: *mut mi_heap_t, count: size_t, size: size_t) -> *mut c_void;
    pub fn mi_heap_realloc(heap: *mut mi_heap_t, p: *mut c_void, newsize: size_t) -> *mut c_void;
    pub fn mi_heap_malloc_aligned(
        heap: *mut mi_heap_t,
        size: size_t,
        alignment: size_t,
    ) -> *mut c_void;
}

// ── Arena management ────────────────────────────────────────────────────────

unsafe extern "C" {
    pub fn mi_reserve_os_memory_ex(
        size: size_t,
        commit: bool,
        allow_large: bool,
        exclusive: bool,
        arena_id: *mut mi_arena_id_t,
    ) -> c_int;
    pub fn mi_manage_os_memory_ex(
        start: *mut c_void,
        size: size_t,
        is_committed: bool,
        is_pinned: bool,
        is_zero: bool,
        numa_node: c_int,
        exclusive: bool,
        arena_id: *mut mi_arena_id_t,
    ) -> bool;
    pub fn mi_reserve_huge_os_pages_interleave(
        pages: size_t,
        numa_nodes: size_t,
        timeout_msecs: size_t,
    ) -> c_int;
    pub fn mi_reserve_huge_os_pages_at(
        pages: size_t,
        numa_node: c_int,
        timeout_msecs: size_t,
    ) -> c_int;
    pub fn mi_arena_min_alignment() -> size_t;
    pub fn mi_arena_min_size() -> size_t;
    pub fn mi_arena_max_object_size() -> size_t;
    pub fn mi_heap_new_in_arena(arena_id: mi_arena_id_t) -> *mut mi_heap_t;
}

// ── Options ─────────────────────────────────────────────────────────────────

unsafe extern "C" {
    pub fn mi_option_is_enabled(option: mi_option_t) -> bool;
    pub fn mi_option_enable(option: mi_option_t);
    pub fn mi_option_disable(option: mi_option_t);
    pub fn mi_option_get(option: mi_option_t) -> c_long;
    pub fn mi_option_get_size(option: mi_option_t) -> size_t;
    pub fn mi_option_set(option: mi_option_t, value: c_long);
}

// ── Experimental profiling ─────────────────────────────────────────────────

unsafe extern "C" {
    pub fn mi_heap_profile(heap: *mut mi_heap_t, profiler: *mut mi_profiler_t) -> bool;
    pub fn mi_heap_profile_disable(heap: *mut mi_heap_t);
    pub fn mi_subproc_profile(subproc_id: mi_subproc_id_t, profiler: *mut mi_profiler_t) -> bool;
    pub fn mi_profile(profiler: *mut mi_profiler_t) -> bool;
    pub fn mi_profiler_start(profiler: *mut mi_profiler_t) -> bool;
    pub fn mi_profiler_stop(profiler: *mut mi_profiler_t) -> bool;
}

// ── POSIX-compatible ────────────────────────────────────────────────────────

unsafe extern "C" {
    pub fn mi_posix_memalign(p: *mut *mut c_void, alignment: size_t, size: size_t) -> c_int;
    pub fn mi_memalign(alignment: size_t, size: size_t) -> *mut c_void;
    pub fn mi_malloc_size(p: *const c_void) -> size_t;
    pub fn mi_malloc_usable_size(p: *const c_void) -> size_t;
}

// ── Statistics ──────────────────────────────────────────────────────────────

unsafe extern "C" {
    pub fn mi_stats_get_json(buf_size: size_t, buf: *mut c_char) -> *mut c_char;
    pub fn mi_stats_print_out(out: Option<mi_output_fun>, arg: *mut c_void);
    pub fn mi_stats_reset();
    pub fn mi_heap_stats_get_json(
        heap: *mut mi_heap_t,
        buf_size: size_t,
        buf: *mut c_char,
    ) -> *mut c_char;
    pub fn mi_heap_stats_print_out(
        heap: *mut mi_heap_t,
        out: Option<mi_output_fun>,
        arg: *mut c_void,
    );
}
