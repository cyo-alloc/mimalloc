//! Low-level FFI bindings to [mimalloc](https://github.com/microsoft/mimalloc) V3 (v3.5.0).
//!
//! For a safe wrapper, use the `rustfs-mimalloc` crate.

#![no_std]
#![allow(non_camel_case_types, unsafe_op_in_unsafe_fn)]

use core::ffi::c_void;

// ── Type aliases ────────────────────────────────────────────────────────────

pub type c_char = i8;
pub type c_int = i32;
pub type c_long = i64;
pub type size_t = usize;

// ── Opaque types ────────────────────────────────────────────────────────────

#[repr(C)]
pub struct mi_heap_t {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct mi_theap_t {
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
// Kept in sync with mimalloc V3.5.0 `mi_option_e` in `mimalloc.h`.

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum mi_option_t {
    mi_option_show_errors = 0,
    mi_option_show_stats = 1,
    mi_option_verbose = 2,
    mi_option_arena_eager_commit = 4,
    mi_option_purge_decommits = 5,
    mi_option_allow_large_os_pages = 6,
    mi_option_reserve_huge_os_pages = 7,
    mi_option_reserve_huge_os_pages_at = 8,
    mi_option_reserve_os_memory = 9,
    mi_option_purge_delay = 15,
    mi_option_use_numa_nodes = 16,
    mi_option_disallow_os_alloc = 17,
    mi_option_os_tag = 18,
    mi_option_max_errors = 19,
    mi_option_max_warnings = 20,
    mi_option_destroy_on_exit = 22,
    mi_option_arena_reserve = 23,
    mi_option_arena_purge_mult = 24,
    mi_option_disallow_arena_alloc = 26,
    mi_option_retry_on_oom = 27,
    mi_option_guarded_min = 29,
    mi_option_guarded_max = 30,
    mi_option_guarded_precise = 31,
    mi_option_guarded_sample_rate = 32,
    mi_option_guarded_sample_seed = 33,
    mi_option_generic_collect = 34,
    mi_option_page_reclaim_on_free = 35,
    mi_option_page_full_retain = 36,
    mi_option_page_max_candidates = 37,
    mi_option_max_vabits = 38,
    mi_option_pagemap_commit = 39,
    mi_option_page_commit_on_demand = 40,
    mi_option_page_max_reclaim = 41,
    mi_option_page_cross_thread_max_reclaim = 42,
    mi_option_allow_thp = 43,
    mi_option_minimal_purge_size = 44,
    mi_option_arena_max_object_size = 45,
    mi_option_arena_is_numa_local = 46,
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
pub type mi_block_visit_fun = unsafe extern "C" fn(
    heap: *const mi_heap_t,
    area: *const mi_heap_area_t,
    block: *mut c_void,
    block_size: size_t,
    arg: *mut c_void,
) -> bool;
pub type mi_heap_visit_fun = unsafe extern "C" fn(heap: *mut mi_heap_t, arg: *mut c_void) -> bool;

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
    pub fn mi_zalloc(size: size_t) -> *mut c_void;
    pub fn mi_mallocn(count: size_t, size: size_t) -> *mut c_void;
    pub fn mi_reallocn(p: *mut c_void, count: size_t, size: size_t) -> *mut c_void;
    pub fn mi_usable_size(p: *const c_void) -> size_t;
    pub fn mi_good_size(size: size_t) -> size_t;
    pub fn mi_free_size(p: *mut c_void, size: size_t);
    pub fn mi_free_small(p: *mut c_void);
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
