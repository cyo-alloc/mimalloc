//! Low-level FFI bindings to [mimalloc](https://github.com/microsoft/mimalloc) v3.
//!
//! The build script compiles the bundled mimalloc sources into a static
//! library and links it. For a safe wrapper and the global allocator, use the
//! `cyo-mimalloc` crate.
//!
//! Each function has the name and the signature of its declaration in
//! `mimalloc.h` or `mimalloc-profile.h`, and those headers document it.
//! [`mi_option_t`] documents every runtime option and how to set it.

#![no_std]
#![allow(non_camel_case_types)]
#![warn(missing_docs)]

// ── Type aliases ────────────────────────────────────────────────────────────

pub use core::ffi::{c_char, c_int, c_long, c_void};

/// The C `size_t` type.
pub type size_t = usize;

// ── Constants ──────────────────────────────────────────────────────────────

/// The largest number of bytes of profiler data that mimalloc stores with a
/// sampled allocation.
pub const MI_PROFILE_SAMPLE_DATA_MAX_SIZE: size_t = 1024;

// ── Opaque types ────────────────────────────────────────────────────────────

/// A mimalloc heap.
///
/// You only handle it through pointers that mimalloc returns.
#[repr(C)]
pub struct mi_heap_t {
    _opaque: [u8; 0],
}

/// The identifier of a mimalloc subprocess.
///
/// Its field is private. Pass the value only to mimalloc functions.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mi_subproc_id_t {
    _id: *mut c_void,
}

/// The identifier of an arena.
///
/// Pass the value only to mimalloc functions.
pub type mi_arena_id_t = *mut c_void;

// ── Option enum ─────────────────────────────────────────────────────────────
//
// Kept in sync with `mi_option_e` in `mimalloc.h`.

/// A mimalloc runtime option.
///
/// You can set an option in three places. A later one takes precedence over an
/// earlier one:
///
/// 1. At build time, through a `MI_DEFAULT_*` variable in the build
///    environment. Only the options whose entry below names a variable accept
///    one.
/// 2. In the environment of the running process, as `MIMALLOC_<NAME>`.
///    `<NAME>` is the variant name without the `mi_option_` prefix, in upper
///    case. mimalloc reads these variables once, when the process starts.
/// 3. From code, with `mi_option_set` (or `MiMalloc::option_set` in
///    `cyo-mimalloc`).
///
/// Setting an option from code only has an effect if mimalloc reads it again
/// afterwards. Each variant says when mimalloc reads it:
///
/// - **on use**: every time mimalloc needs the value. You can change it at any
///   time.
/// - **per thread**: when a thread first allocates. A change applies to threads
///   that start afterwards.
/// - **at startup**: once, before `main`. Setting it from code has no effect.
///   Use the environment variable or the build-time default instead.
/// - **at exit**: when the process ends.
///
/// In the environment, an option measured in KiB accepts a `K`, `M`, `G` or `T`
/// suffix, as in `MIMALLOC_ARENA_RESERVE=4GiB`. Through `mi_option_set` and
/// `MI_DEFAULT_*`, the value is a plain number of KiB. `mi_option_get_size`
/// returns the value in bytes. A boolean option accepts `1`/`0`,
/// `true`/`false`, `on`/`off` and `yes`/`no`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum mi_option_t {
    /// Whether mimalloc prints error messages to stderr.
    ///
    /// The default is 0, or 1 in a build with `MI_DEBUG`. mimalloc reads it on
    /// use.
    mi_option_show_errors = 0,
    /// Whether mimalloc prints statistics to stderr when the process exits.
    ///
    /// The default is 0. mimalloc reads it at exit.
    mi_option_show_stats = 1,
    /// Whether mimalloc prints verbose messages. These include the option
    /// values at startup and the statistics at exit.
    ///
    /// The default is 0. `MI_DEFAULT_VERBOSE` sets the default at build time.
    /// mimalloc reads it on use.
    mi_option_verbose = 2,
    /// Whether mimalloc commits all the memory of a new arena when it creates
    /// the arena.
    ///
    /// 0 commits the memory on demand instead. 1 commits all of it, and 2 does
    /// so only on a system that overcommits, such as Linux. Committing memory
    /// does not raise the resident set. The default is 2.
    /// `MI_DEFAULT_ARENA_EAGER_COMMIT` sets the default at build time.
    /// mimalloc reads it on use, when it creates an arena.
    mi_option_arena_eager_commit = 4,
    /// How mimalloc gives unused memory back to the OS: 1 decommits it, and 0
    /// resets it.
    ///
    /// On Linux, decommitting uses `MADV_DONTNEED` and lowers the resident set
    /// immediately. Resetting uses `MADV_FREE`, and the kernel reclaims the
    /// memory when it needs it. The default is 1. mimalloc reads it on use.
    mi_option_purge_decommits = 5,
    /// Whether mimalloc uses explicit large OS pages (2 MiB, `MAP_HUGETLB`)
    /// when they are available.
    ///
    /// The kernel only has these pages if `vm.nr_hugepages` is set, and
    /// mimalloc cannot purge them. The default is 0.
    /// `MI_DEFAULT_ALLOW_LARGE_OS_PAGES` sets the default at build time.
    /// mimalloc reads it on use.
    mi_option_allow_large_os_pages = 6,
    /// The number of 1 GiB huge pages that mimalloc reserves at startup,
    /// spread over the NUMA nodes.
    ///
    /// The default is 0. `MI_DEFAULT_RESERVE_HUGE_OS_PAGES` sets the default at
    /// build time. mimalloc reads it at startup. From code, call
    /// `mi_reserve_huge_os_pages_interleave` instead.
    mi_option_reserve_huge_os_pages = 7,
    /// The NUMA node on which mimalloc reserves all the huge pages of
    /// `mi_option_reserve_huge_os_pages`, or -1 to spread them over all nodes.
    ///
    /// The default is -1. mimalloc reads it at startup. From code, call
    /// `mi_reserve_huge_os_pages_at` instead.
    mi_option_reserve_huge_os_pages_at = 8,
    /// The amount of memory, in KiB, that mimalloc reserves as an arena at
    /// startup.
    ///
    /// The default is 0. `MI_DEFAULT_RESERVE_OS_MEMORY` sets the default at
    /// build time. mimalloc reads it at startup. From code, call
    /// `mi_reserve_os_memory_ex` instead.
    mi_option_reserve_os_memory = 9,
    /// The delay in milliseconds before mimalloc purges unused memory (see
    /// `mi_option_purge_decommits`).
    ///
    /// 0 purges immediately, and -1 never purges. The default is 1000. mimalloc
    /// reads it on use.
    mi_option_purge_delay = 15,
    /// The largest number of NUMA nodes that mimalloc uses, or 0 to detect
    /// them.
    ///
    /// Setting 1 can help in a virtual machine that reports NUMA incorrectly.
    /// The default is 0. mimalloc reads it at startup. It has no build-time
    /// default, so set `MIMALLOC_USE_NUMA_NODES` instead.
    mi_option_use_numa_nodes = 16,
    /// Whether mimalloc allocates only from arenas reserved in advance, and
    /// never from the OS.
    ///
    /// The default is 0. mimalloc reads it on use.
    mi_option_disallow_os_alloc = 17,
    /// The memory tag that mimalloc gives its memory on macOS.
    ///
    /// Only macOS uses it.
    mi_option_os_tag = 18,
    /// The largest number of error messages that mimalloc prints.
    ///
    /// The default is 32. mimalloc reads it at startup.
    mi_option_max_errors = 19,
    /// The largest number of warning messages that mimalloc prints.
    ///
    /// The default is 32. mimalloc reads it at startup.
    mi_option_max_warnings = 20,
    /// Whether mimalloc releases all its OS memory when the process exits,
    /// even memory that other code may still use afterwards.
    ///
    /// The default is 0. mimalloc reads it at exit.
    mi_option_destroy_on_exit = 22,
    /// The size, in KiB, of each arena that mimalloc reserves from the OS.
    ///
    /// The default is 1 GiB. `MI_DEFAULT_ARENA_RESERVE` sets the default at
    /// build time. mimalloc reads it on use, when it creates an arena.
    mi_option_arena_reserve = 23,
    /// The multiplier that mimalloc applies to `mi_option_purge_delay` when it
    /// purges arena memory.
    ///
    /// The default is 4. mimalloc reads it on use.
    mi_option_arena_purge_mult = 24,
    /// Whether mimalloc allocates from an arena only if you request that arena
    /// by its id.
    ///
    /// The default is 0. `MI_DEFAULT_DISALLOW_ARENA_ALLOC` sets the default at
    /// build time. mimalloc reads it on use.
    mi_option_disallow_arena_alloc = 26,
    /// How many milliseconds mimalloc retries an allocation after the OS runs
    /// out of memory, or 0 to not retry.
    ///
    /// Only Windows uses it. The default is 400.
    mi_option_retry_on_oom = 27,
    /// The smallest object size that guarded allocation guards.
    ///
    /// It only has an effect in a build with `MI_GUARDED`, which this crate
    /// does not enable.
    mi_option_guarded_min = 29,
    /// The largest object size that guarded allocation guards.
    ///
    /// It only has an effect in a build with `MI_GUARDED`, which this crate
    /// does not enable.
    mi_option_guarded_max = 30,
    /// Whether guarded allocation places each block directly against its guard
    /// page.
    ///
    /// It only has an effect in a build with `MI_GUARDED`, which this crate
    /// does not enable.
    mi_option_guarded_precise = 31,
    /// The rate N at which guarded allocation guards 1 in N allocations.
    ///
    /// It only has an effect in a build with `MI_GUARDED`, which this crate
    /// does not enable.
    mi_option_guarded_sample_rate = 32,
    /// The random seed that guarded allocation uses for sampling.
    ///
    /// It only has an effect in a build with `MI_GUARDED`, which this crate
    /// does not enable.
    mi_option_guarded_sample_seed = 33,
    /// The number of allocations on the slow path after which mimalloc
    /// collects a thread's heap.
    ///
    /// The default is 10000. mimalloc reads it on use.
    mi_option_generic_collect = 34,
    /// Whether mimalloc reclaims an abandoned page when a block in it is freed.
    ///
    /// -1 never reclaims it. 0 reclaims it only into the thread that owned it,
    /// and 1 reclaims it into any thread. The default is 0. mimalloc reads it
    /// per thread and on use.
    mi_option_page_reclaim_on_free = 35,
    /// The number of empty pages that each thread keeps in its free queues, or
    /// -1 to never abandon pages.
    ///
    /// The default is 2. mimalloc reads it per thread.
    mi_option_page_full_retain = 36,
    /// The largest number of pages that mimalloc searches for the best fit.
    ///
    /// The default is 4. mimalloc reads it on use.
    mi_option_page_max_candidates = 37,
    /// The largest number of usable virtual address bits, or 0 to detect it.
    ///
    /// The default is 0. mimalloc reads it at startup.
    mi_option_max_vabits = 38,
    /// Whether mimalloc commits the whole page map at startup, instead of on
    /// demand.
    ///
    /// The default is 0. `MI_DEFAULT_PAGEMAP_COMMIT` sets the default at build
    /// time. mimalloc reads it at startup.
    mi_option_pagemap_commit = 39,
    /// Whether mimalloc commits pages on demand: 0 = no, 1 = yes, 2 = only on a
    /// system that does not overcommit.
    ///
    /// The default is 0. mimalloc reads it on use.
    mi_option_page_commit_on_demand = 40,
    /// A limit on reclaiming a thread's own abandoned pages, or -1 for no
    /// limit.
    ///
    /// A thread that already owns this many pages of a size class no longer
    /// reclaims its abandoned pages of that class. The default is -1. `MI_DEFAULT_PAGE_MAX_RECLAIM` sets the default at
    /// build time. mimalloc reads it on use.
    mi_option_page_max_reclaim = 41,
    /// A limit on reclaiming pages that other threads abandoned.
    ///
    /// A thread that already owns this many pages of a size class no longer
    /// reclaims pages of that class from other threads. The default is 32. `MI_DEFAULT_PAGE_CROSS_THREAD_MAX_RECLAIM` sets the
    /// default at build time. mimalloc reads it on use.
    mi_option_page_cross_thread_max_reclaim = 42,
    /// Whether mimalloc uses transparent huge pages (THP).
    ///
    /// 0 disables THP for the whole process, and 1 allows it. 2 allows it and
    /// raises `mi_option_minimal_purge_size` to 2 MiB, so that purging does not
    /// split huge pages. The default is 2. `MI_DEFAULT_ALLOW_THP` sets the
    /// default at build time. mimalloc disables THP for the process at startup,
    /// and reads the option on use to decide whether to ask for huge pages.
    mi_option_allow_thp = 43,
    /// The smallest amount of memory, in KiB, that mimalloc purges at once.
    ///
    /// 0 picks 64 KiB, or 2 MiB when THP is available and
    /// `mi_option_allow_thp` is 2. The default is 0. mimalloc reads it on use.
    mi_option_minimal_purge_size = 44,
    /// The size, in KiB, of the largest object that mimalloc allocates from an
    /// arena.
    ///
    /// mimalloc allocates a larger object directly from the OS. The default is
    /// 2 GiB. `MI_DEFAULT_ARENA_MAX_OBJECT_SIZE` sets the default at build
    /// time. mimalloc reads it on use.
    mi_option_arena_max_object_size = 45,
    /// Whether mimalloc ties a new arena to the NUMA node of the thread that
    /// creates it.
    ///
    /// The default is 0. mimalloc reads it on use.
    mi_option_arena_is_numa_local = 46,
    /// Whether mimalloc merges a thread heap's statistics into its parent heap
    /// on every collection.
    ///
    /// The default is 1. `MI_DEFAULT_COLLECT_MERGES_STATS` sets the default at
    /// build time. mimalloc reads it on use.
    mi_option_collect_merges_stats = 47,
}

// ── Heap area (for visiting blocks) ─────────────────────────────────────────

/// An area of a heap that holds blocks of a single size.
#[repr(C)]
pub struct mi_heap_area_t {
    /// The start of the area.
    pub blocks: *mut c_void,
    /// The number of bytes of address space reserved for the area.
    pub reserved: size_t,
    /// The number of bytes of the area that are committed.
    pub committed: size_t,
    /// The number of allocated blocks in the area.
    pub used: size_t,
    /// The size of each block in bytes.
    pub block_size: size_t,
    /// The size of each block in bytes, including padding and metadata.
    pub full_block_size: size_t,
    /// Reserved for mimalloc.
    pub reserved1: *mut c_void,
}

// ── Callback types ──────────────────────────────────────────────────────────

/// A function that receives mimalloc's text output.
///
/// mimalloc passes each piece of output as a NUL-terminated string in `msg`,
/// and passes back the `arg` that you registered.
pub type mi_output_fun = unsafe extern "C" fn(msg: *const c_char, arg: *mut c_void);
/// A function that mimalloc calls when an error occurs, with the `errno`
/// value of the error in `err`.
pub type mi_error_fun = unsafe extern "C" fn(err: c_int, arg: *mut c_void);
/// A function that mimalloc calls regularly, so that you can free memory
/// that you deferred freeing.
pub type mi_deferred_free_fun = unsafe extern "C" fn(force: bool, heartbeat: u64, arg: *mut c_void);
/// A function that mimalloc calls on each sampled allocation.
pub type mi_profiler_on_alloc_fun = unsafe extern "C" fn(
    profiler: *mut mi_profiler_t,
    profiler_data: *mut mi_profiler_sample_data_t,
    ptr: *mut c_void,
    requested_size: size_t,
    bytes_sample_rate: size_t,
    bytes_since_last_sample: u64,
    heap: *const mi_heap_t,
) -> size_t;
/// A function that mimalloc calls when it reallocates a sampled allocation in
/// place.
pub type mi_profiler_on_realloc_inplace_fun = unsafe extern "C" fn(
    profiler: *mut mi_profiler_t,
    profiler_data: *mut mi_profiler_sample_data_t,
    ptr: *mut c_void,
    old_size: size_t,
    heap: *const mi_heap_t,
) -> size_t;
/// A function that mimalloc calls when a sampled allocation is freed.
pub type mi_profiler_on_free_fun = unsafe extern "C" fn(
    profiler: *mut mi_profiler_t,
    profiler_data: *mut mi_profiler_sample_data_t,
    ptr: *mut c_void,
    heap: *const mi_heap_t,
);
/// A function that mimalloc calls for each area of a heap, and for each
/// block when you ask it to visit blocks.
///
/// Return `false` to stop the visit.
pub type mi_block_visit_fun = unsafe extern "C" fn(
    heap: *const mi_heap_t,
    area: *const mi_heap_area_t,
    block: *mut c_void,
    block_size: size_t,
    arg: *mut c_void,
) -> bool;
/// A function that mimalloc calls for each heap of a subprocess.
///
/// Return `false` to stop the visit.
pub type mi_heap_visit_fun = unsafe extern "C" fn(heap: *mut mi_heap_t, arg: *mut c_void) -> bool;

// ── Profiling ───────────────────────────────────────────────────────────────

/// The profiler data that mimalloc stores with a sampled allocation.
///
/// mimalloc stores it only if the profiler's `on_free` is set.
#[repr(C)]
pub struct mi_profiler_sample_data_t {
    /// The size of `user_data` in bytes, equal to the profiler's
    /// `sample_data_size`.
    pub user_data_size: size_t,
    /// The start of the user data. The data can be longer than this array,
    /// up to [`MI_PROFILE_SAMPLE_DATA_MAX_SIZE`] bytes.
    pub user_data: [*mut c_void; 1],
}

/// A table of profiling callbacks. mimalloc's profiling support is
/// experimental.
///
/// mimalloc may copy the table and read it from several threads at once, so
/// do not change it after you register it. Every field can be null or 0.
#[repr(C)]
pub struct mi_profiler_t {
    /// Reserved for mimalloc.
    pub reserved: *mut c_void,
    /// The number of bytes of data that mimalloc stores with each sampled
    /// allocation, or 0 for none.
    pub sample_data_size: size_t,
    /// The initial sample rate in bytes, at least 1. `on_alloc` can change it.
    pub initial_sample_rate: size_t,
    /// The function that mimalloc calls on a sampled allocation, possibly
    /// from several threads at once.
    pub on_alloc: Option<mi_profiler_on_alloc_fun>,
    /// The function that mimalloc calls when a sampled allocation is freed,
    /// possibly from several threads at once.
    pub on_free: Option<mi_profiler_on_free_fun>,
    /// The function that mimalloc calls when it reallocates a sampled
    /// allocation in place. mimalloc v3.5 does not call it yet.
    pub on_realloc_inplace: Option<mi_profiler_on_realloc_inplace_fun>,
}

// ── Standard malloc interface ───────────────────────────────────────────────

#[allow(missing_docs)]
unsafe extern "C" {
    pub fn mi_malloc(size: size_t) -> *mut c_void;
    pub fn mi_calloc(count: size_t, size: size_t) -> *mut c_void;
    pub fn mi_realloc(p: *mut c_void, newsize: size_t) -> *mut c_void;
    pub fn mi_free(p: *mut c_void);
    pub fn mi_strdup(s: *const c_char) -> *mut c_char;
}

// ── Extended allocation ─────────────────────────────────────────────────────

#[allow(missing_docs)]
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

#[allow(missing_docs)]
unsafe extern "C" {
    pub fn mi_malloc_aligned(size: size_t, alignment: size_t) -> *mut c_void;
    pub fn mi_zalloc_aligned(size: size_t, alignment: size_t) -> *mut c_void;
    pub fn mi_calloc_aligned(count: size_t, size: size_t, alignment: size_t) -> *mut c_void;
    pub fn mi_realloc_aligned(p: *mut c_void, newsize: size_t, alignment: size_t) -> *mut c_void;
}

// ── Process & thread lifecycle ──────────────────────────────────────────────

#[allow(missing_docs)]
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

#[allow(missing_docs)]
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

#[allow(missing_docs)]
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

#[allow(missing_docs)]
unsafe extern "C" {
    pub fn mi_option_is_enabled(option: mi_option_t) -> bool;
    pub fn mi_option_enable(option: mi_option_t);
    pub fn mi_option_disable(option: mi_option_t);
    pub fn mi_option_get(option: mi_option_t) -> c_long;
    pub fn mi_option_get_size(option: mi_option_t) -> size_t;
    pub fn mi_option_set(option: mi_option_t, value: c_long);
}

// ── Experimental profiling ─────────────────────────────────────────────────

#[allow(missing_docs)]
unsafe extern "C" {
    pub fn mi_heap_profile(heap: *mut mi_heap_t, profiler: *mut mi_profiler_t) -> bool;
    pub fn mi_heap_profile_disable(heap: *mut mi_heap_t);
    pub fn mi_subproc_profile(subproc_id: mi_subproc_id_t, profiler: *mut mi_profiler_t) -> bool;
    pub fn mi_profile(profiler: *mut mi_profiler_t) -> bool;
    pub fn mi_profiler_start(profiler: *mut mi_profiler_t) -> bool;
    pub fn mi_profiler_stop(profiler: *mut mi_profiler_t) -> bool;
}

// ── POSIX-compatible ────────────────────────────────────────────────────────

#[allow(missing_docs)]
unsafe extern "C" {
    pub fn mi_posix_memalign(p: *mut *mut c_void, alignment: size_t, size: size_t) -> c_int;
    pub fn mi_memalign(alignment: size_t, size: size_t) -> *mut c_void;
    pub fn mi_malloc_size(p: *const c_void) -> size_t;
    pub fn mi_malloc_usable_size(p: *const c_void) -> size_t;
}

// ── Statistics ──────────────────────────────────────────────────────────────

#[allow(missing_docs)]
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
