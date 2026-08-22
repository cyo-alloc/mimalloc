//! Heap and arena operations for advanced memory management.

#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::c_void;
use core::ffi::CStr;
use core::ptr::NonNull;

// ── Error type ──────────────────────────────────────────────────────────────

/// Error returned by arena operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArenaError {
    /// The OS call failed (e.g., out of memory, invalid parameters).
    Failed,
}

// ── Heap ────────────────────────────────────────────────────────────────────

/// A handle to a mimalloc heap.
///
/// Allocations from a heap can be freed from any thread.
/// Dropping a `Heap` moves its live blocks to the main heap (via `mi_heap_delete`).
pub struct Heap {
    ptr: NonNull<rustfs_mimalloc_sys::mi_heap_t>,
}

unsafe impl Send for Heap {}
unsafe impl Sync for Heap {}

impl Heap {
    /// Create a new heap. Returns `None` on OOM.
    pub fn new() -> Option<Self> {
        NonNull::new(unsafe { rustfs_mimalloc_sys::mi_heap_new() }).map(|ptr| Heap { ptr })
    }

    /// Create a heap that allocates exclusively from the given arena.
    pub fn new_in_arena(arena_id: ArenaId) -> Option<Self> {
        NonNull::new(unsafe { rustfs_mimalloc_sys::mi_heap_new_in_arena(arena_id.0) })
            .map(|ptr| Heap { ptr })
    }

    /// Get the main heap.
    pub fn main() -> Self {
        let ptr = unsafe { rustfs_mimalloc_sys::mi_heap_main() };
        Heap {
            ptr: NonNull::new(ptr).expect("mi_heap_main returned null"),
        }
    }

    /// Get the heap that owns `ptr`.
    ///
    /// # Safety
    /// `ptr` must be a valid mimalloc-allocated pointer.
    pub unsafe fn heap_of(ptr: *const u8) -> Option<Self> {
        NonNull::new(rustfs_mimalloc_sys::mi_heap_of(ptr as *const c_void)).map(|ptr| Heap { ptr })
    }

    /// Check if this heap contains `ptr`.
    ///
    /// # Safety
    /// `ptr` must be valid.
    pub unsafe fn contains(&self, ptr: *const u8) -> bool {
        rustfs_mimalloc_sys::mi_heap_contains(self.ptr.as_ptr(), ptr as *const c_void)
    }

    /// Allocate `size` bytes from this heap.
    ///
    /// # Safety
    /// The returned pointer must be freed with `mi_free` (cross-heap frees are allowed).
    pub unsafe fn malloc(&self, size: usize) -> *mut u8 {
        rustfs_mimalloc_sys::mi_heap_malloc(self.ptr.as_ptr(), size) as *mut u8
    }

    /// Allocate zero-initialized memory from this heap.
    ///
    /// # Safety
    /// The returned pointer must be freed with `mi_free`.
    pub unsafe fn zalloc(&self, size: usize) -> *mut u8 {
        rustfs_mimalloc_sys::mi_heap_zalloc(self.ptr.as_ptr(), size) as *mut u8
    }

    /// Allocate aligned memory from this heap.
    ///
    /// # Safety
    /// The returned pointer must be freed with `mi_free`.
    pub unsafe fn malloc_aligned(&self, size: usize, alignment: usize) -> *mut u8 {
        rustfs_mimalloc_sys::mi_heap_malloc_aligned(self.ptr.as_ptr(), size, alignment) as *mut u8
    }

    /// Reallocate memory from this heap.
    ///
    /// # Safety
    /// `ptr` must be a valid mimalloc pointer. The returned pointer must be freed with `mi_free`.
    pub unsafe fn realloc(&self, ptr: *mut u8, new_size: usize) -> *mut u8 {
        rustfs_mimalloc_sys::mi_heap_realloc(self.ptr.as_ptr(), ptr as *mut c_void, new_size)
            as *mut u8
    }

    /// Delete this heap, moving live blocks to the main heap.
    /// Consumes `self` without running `Drop`.
    pub fn delete(self) {
        unsafe { rustfs_mimalloc_sys::mi_heap_delete(self.ptr.as_ptr()) }
        core::mem::forget(self);
    }

    /// Destroy this heap, freeing all live blocks.
    ///
    /// # Safety
    /// All pointers from this heap become dangling.
    pub unsafe fn destroy(self) {
        rustfs_mimalloc_sys::mi_heap_destroy(self.ptr.as_ptr());
        core::mem::forget(self);
    }

    /// Force garbage collection on this heap.
    pub fn collect(&self, force: bool) {
        unsafe { rustfs_mimalloc_sys::mi_heap_collect(self.ptr.as_ptr(), force) }
    }

    /// Allocation statistics for this heap as JSON. Returns empty string on failure.
    pub fn stats_json(&self) -> String {
        unsafe {
            let ptr = rustfs_mimalloc_sys::mi_heap_stats_get_json(
                self.ptr.as_ptr(),
                0,
                core::ptr::null_mut(),
            );
            if ptr.is_null() {
                return String::new();
            }
            let result = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            rustfs_mimalloc_sys::mi_free(ptr as *mut c_void);
            result
        }
    }

    /// Allocation statistics for this heap in mimalloc's human-readable text format.
    pub fn stats_print(&self) -> String {
        crate::extended::collect_mimalloc_output(|out, arg| unsafe {
            rustfs_mimalloc_sys::mi_heap_stats_print_out(self.ptr.as_ptr(), out, arg);
        })
    }

    /// Raw pointer to the underlying `mi_heap_t`.
    pub fn as_ptr(&self) -> *mut rustfs_mimalloc_sys::mi_heap_t {
        self.ptr.as_ptr()
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        unsafe { rustfs_mimalloc_sys::mi_heap_delete(self.ptr.as_ptr()) }
    }
}

// ── Arena ───────────────────────────────────────────────────────────────────

/// Arena identifier for managing memory regions.
#[derive(Debug, Clone, Copy)]
pub struct ArenaId(rustfs_mimalloc_sys::mi_arena_id_t);

unsafe impl Send for ArenaId {}
unsafe impl Sync for ArenaId {}

/// Reserve OS memory as an exclusive arena.
pub fn reserve_os_memory(
    size: usize,
    commit: bool,
    allow_large: bool,
    exclusive: bool,
) -> Result<ArenaId, ArenaError> {
    let mut id = core::ptr::null_mut();
    let rc = unsafe {
        rustfs_mimalloc_sys::mi_reserve_os_memory_ex(size, commit, allow_large, exclusive, &mut id)
    };
    if rc == 0 {
        Ok(ArenaId(id))
    } else {
        Err(ArenaError::Failed)
    }
}

/// Manage an existing memory region as an arena.
///
/// # Safety
/// `start` must point to at least `size` valid bytes that outlive the arena.
pub unsafe fn manage_os_memory(
    start: *mut u8,
    size: usize,
    is_committed: bool,
    is_pinned: bool,
    is_zero: bool,
    numa_node: i32,
    exclusive: bool,
) -> Result<ArenaId, ArenaError> {
    let mut id = core::ptr::null_mut();
    let ok = rustfs_mimalloc_sys::mi_manage_os_memory_ex(
        start as *mut c_void,
        size,
        is_committed,
        is_pinned,
        is_zero,
        numa_node,
        exclusive,
        &mut id,
    );
    if ok {
        Ok(ArenaId(id))
    } else {
        Err(ArenaError::Failed)
    }
}

/// Minimum alignment for arena allocations.
#[inline]
pub fn arena_min_alignment() -> usize {
    unsafe { rustfs_mimalloc_sys::mi_arena_min_alignment() }
}

/// Minimum size for arena allocations.
#[inline]
pub fn arena_min_size() -> usize {
    unsafe { rustfs_mimalloc_sys::mi_arena_min_size() }
}

/// Maximum object size for arena allocations.
#[inline]
pub fn arena_max_object_size() -> usize {
    unsafe { rustfs_mimalloc_sys::mi_arena_max_object_size() }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heap_create_delete() {
        let heap = Heap::new().expect("heap::new failed");
        heap.delete();
    }

    #[test]
    fn heap_alloc_free() {
        let heap = Heap::new().unwrap();
        unsafe {
            let ptr = heap.malloc(128);
            assert!(!ptr.is_null());
            core::ptr::write_bytes(ptr, 0xCD, 128);
            rustfs_mimalloc_sys::mi_free(ptr as *mut c_void);
        }
        heap.delete();
    }

    #[test]
    fn heap_aligned_alloc() {
        let heap = Heap::new().unwrap();
        unsafe {
            for pow in 0..=12 {
                let align = 1usize << pow;
                let ptr = heap.malloc_aligned(64, align);
                assert!(!ptr.is_null());
                assert_eq!(ptr as usize % align, 0, "align={align}");
                rustfs_mimalloc_sys::mi_free(ptr as *mut c_void);
            }
        }
        heap.delete();
    }

    #[test]
    fn arena_min_values_are_sane() {
        let a = arena_min_alignment();
        assert!(a > 0 && a.is_power_of_two());
    }

    #[test]
    fn heap_stats_are_available() {
        let heap = Heap::new().unwrap();
        assert!(!heap.stats_json().is_empty());
        assert!(!heap.stats_print().is_empty());
        heap.delete();
    }
}
