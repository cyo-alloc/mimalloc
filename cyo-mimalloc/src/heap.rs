//! Heap and arena operations for advanced memory management.

use core::ffi::c_void;
use core::fmt;
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
    ptr: NonNull<cyo_mimalloc_sys::mi_heap_t>,
    owned: bool,
}

unsafe impl Send for Heap {}
unsafe impl Sync for Heap {}

impl Heap {
    /// Create a new heap. Returns `None` on OOM.
    pub fn new() -> Option<Self> {
        NonNull::new(unsafe { cyo_mimalloc_sys::mi_heap_new() }).map(Self::owned)
    }

    /// Create a heap that allocates exclusively from the given arena.
    pub fn new_in_arena(arena_id: ArenaId) -> Option<Self> {
        NonNull::new(unsafe { cyo_mimalloc_sys::mi_heap_new_in_arena(arena_id.0) }).map(Self::owned)
    }

    /// Get the main heap.
    pub fn main() -> Self {
        let ptr = unsafe { cyo_mimalloc_sys::mi_heap_main() };
        Self::borrowed(NonNull::new(ptr).expect("mi_heap_main returned null"))
    }

    /// Get the heap that owns `ptr`.
    ///
    /// # Safety
    /// `ptr` must be a valid mimalloc-allocated pointer.
    pub unsafe fn heap_of(ptr: *const u8) -> Option<Self> {
        NonNull::new(unsafe { cyo_mimalloc_sys::mi_heap_of(ptr as *const c_void) })
            .map(Self::borrowed)
    }

    /// Check if this heap contains `ptr`.
    ///
    /// # Safety
    /// `ptr` must be valid.
    pub unsafe fn contains(&self, ptr: *const u8) -> bool {
        unsafe { cyo_mimalloc_sys::mi_heap_contains(self.ptr.as_ptr(), ptr as *const c_void) }
    }

    /// Allocate `size` bytes from this heap.
    ///
    /// # Safety
    /// The returned pointer must be freed with `mi_free` (cross-heap frees are allowed).
    pub unsafe fn malloc(&self, size: usize) -> *mut u8 {
        unsafe { cyo_mimalloc_sys::mi_heap_malloc(self.ptr.as_ptr(), size) as *mut u8 }
    }

    /// Allocate zero-initialized memory from this heap.
    ///
    /// # Safety
    /// The returned pointer must be freed with `mi_free`.
    pub unsafe fn zalloc(&self, size: usize) -> *mut u8 {
        unsafe { cyo_mimalloc_sys::mi_heap_zalloc(self.ptr.as_ptr(), size) as *mut u8 }
    }

    /// Allocate aligned memory from this heap.
    ///
    /// # Safety
    /// The returned pointer must be freed with `mi_free`.
    pub unsafe fn malloc_aligned(&self, size: usize, alignment: usize) -> *mut u8 {
        unsafe {
            cyo_mimalloc_sys::mi_heap_malloc_aligned(self.ptr.as_ptr(), size, alignment) as *mut u8
        }
    }

    /// Reallocate memory from this heap.
    ///
    /// # Safety
    /// `ptr` must be a valid mimalloc pointer. The returned pointer must be freed with `mi_free`.
    pub unsafe fn realloc(&self, ptr: *mut u8, new_size: usize) -> *mut u8 {
        unsafe {
            cyo_mimalloc_sys::mi_heap_realloc(self.ptr.as_ptr(), ptr as *mut c_void, new_size)
                as *mut u8
        }
    }

    /// Delete this heap, moving live blocks to the main heap.
    /// Consumes `self` without running `Drop`. Borrowed heap handles are left untouched.
    pub fn delete(self) {
        if self.owned {
            unsafe { cyo_mimalloc_sys::mi_heap_delete(self.ptr.as_ptr()) }
        }
        core::mem::forget(self);
    }

    /// Destroy this heap, freeing all live blocks.
    ///
    /// # Safety
    /// All pointers from this heap become dangling. Borrowed heap handles are left untouched.
    pub unsafe fn destroy(self) {
        if self.owned {
            unsafe { cyo_mimalloc_sys::mi_heap_destroy(self.ptr.as_ptr()) };
        }
        core::mem::forget(self);
    }

    /// Force garbage collection on this heap.
    pub fn collect(&self, force: bool) {
        unsafe { cyo_mimalloc_sys::mi_heap_collect(self.ptr.as_ptr(), force) }
    }

    /// Write this heap's allocation statistics to `out` as JSON.
    ///
    /// Fails if mimalloc cannot produce the statistics or `out` fails.
    pub fn stats_json(&self, out: &mut (impl fmt::Write + ?Sized)) -> fmt::Result {
        unsafe {
            crate::ffi::write_owned_c_string(
                out,
                cyo_mimalloc_sys::mi_heap_stats_get_json(
                    self.ptr.as_ptr(),
                    0,
                    core::ptr::null_mut(),
                ),
            )
        }
    }

    /// Write this heap's allocation statistics to `out` in mimalloc's
    /// human-readable text format.
    pub fn stats_print(&self, out: &mut dyn fmt::Write) -> fmt::Result {
        crate::ffi::write_output(out, |out, arg| unsafe {
            cyo_mimalloc_sys::mi_heap_stats_print_out(self.ptr.as_ptr(), out, arg);
        })
    }

    /// Raw pointer to the underlying `mi_heap_t`.
    pub fn as_ptr(&self) -> *mut cyo_mimalloc_sys::mi_heap_t {
        self.ptr.as_ptr()
    }

    #[inline]
    fn owned(ptr: NonNull<cyo_mimalloc_sys::mi_heap_t>) -> Self {
        Self { ptr, owned: true }
    }

    #[inline]
    fn borrowed(ptr: NonNull<cyo_mimalloc_sys::mi_heap_t>) -> Self {
        Self { ptr, owned: false }
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        if self.owned {
            unsafe { cyo_mimalloc_sys::mi_heap_delete(self.ptr.as_ptr()) }
        }
    }
}

// ── Arena ───────────────────────────────────────────────────────────────────

/// Arena identifier for managing memory regions.
#[derive(Debug, Clone, Copy)]
pub struct ArenaId(cyo_mimalloc_sys::mi_arena_id_t);

unsafe impl Send for ArenaId {}
unsafe impl Sync for ArenaId {}

/// Reserve `size` bytes of OS memory as an arena.
///
/// This is the in-code counterpart of the `reserve_os_memory` option, which
/// mimalloc only reads at startup. `commit` commits the memory up front,
/// `allow_large` lets it use large OS pages, and `exclusive` keeps the arena
/// out of normal allocation: only heaps created with [`Heap::new_in_arena`]
/// use it.
pub fn reserve_os_memory(
    size: usize,
    commit: bool,
    allow_large: bool,
    exclusive: bool,
) -> Result<ArenaId, ArenaError> {
    let mut id = core::ptr::null_mut();
    let rc = unsafe {
        cyo_mimalloc_sys::mi_reserve_os_memory_ex(size, commit, allow_large, exclusive, &mut id)
    };
    if rc == 0 {
        Ok(ArenaId(id))
    } else {
        Err(ArenaError::Failed)
    }
}

/// Reserve `pages` huge OS pages (1 GiB each), spread over `numa_nodes` NUMA
/// nodes (0 = all of them), giving up after `timeout_msecs` milliseconds.
///
/// This is the in-code counterpart of the `reserve_huge_os_pages` option, which
/// mimalloc only reads at startup. The kernel must have 1 GiB huge pages
/// available.
pub fn reserve_huge_os_pages_interleave(
    pages: usize,
    numa_nodes: usize,
    timeout_msecs: usize,
) -> Result<(), ArenaError> {
    let rc = unsafe {
        cyo_mimalloc_sys::mi_reserve_huge_os_pages_interleave(pages, numa_nodes, timeout_msecs)
    };
    if rc == 0 {
        Ok(())
    } else {
        Err(ArenaError::Failed)
    }
}

/// Reserve `pages` huge OS pages (1 GiB each) on NUMA node `numa_node`,
/// giving up after `timeout_msecs` milliseconds.
///
/// This is the in-code counterpart of the `reserve_huge_os_pages` and
/// `reserve_huge_os_pages_at` options, which mimalloc only reads at startup.
pub fn reserve_huge_os_pages_at(
    pages: usize,
    numa_node: i32,
    timeout_msecs: usize,
) -> Result<(), ArenaError> {
    let rc =
        unsafe { cyo_mimalloc_sys::mi_reserve_huge_os_pages_at(pages, numa_node, timeout_msecs) };
    if rc == 0 {
        Ok(())
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
    let ok = unsafe {
        cyo_mimalloc_sys::mi_manage_os_memory_ex(
            start as *mut c_void,
            size,
            is_committed,
            is_pinned,
            is_zero,
            numa_node,
            exclusive,
            &mut id,
        )
    };
    if ok {
        Ok(ArenaId(id))
    } else {
        Err(ArenaError::Failed)
    }
}

/// Minimum alignment for arena allocations.
#[inline]
pub fn arena_min_alignment() -> usize {
    unsafe { cyo_mimalloc_sys::mi_arena_min_alignment() }
}

/// Minimum size for arena allocations.
#[inline]
pub fn arena_min_size() -> usize {
    unsafe { cyo_mimalloc_sys::mi_arena_min_size() }
}

/// Maximum object size for arena allocations.
#[inline]
pub fn arena_max_object_size() -> usize {
    unsafe { cyo_mimalloc_sys::mi_arena_max_object_size() }
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
            cyo_mimalloc_sys::mi_free(ptr as *mut c_void);
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
                cyo_mimalloc_sys::mi_free(ptr as *mut c_void);
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
    fn reserve_zero_huge_pages_is_ok() {
        assert_eq!(reserve_huge_os_pages_interleave(0, 0, 0), Ok(()));
        assert_eq!(reserve_huge_os_pages_at(0, 0, 0), Ok(()));
    }

    #[test]
    fn heap_stats_are_available() {
        let heap = Heap::new().unwrap();
        let mut json = std::string::String::new();
        heap.stats_json(&mut json).unwrap();
        assert!(json.starts_with('{'));
        let mut text = std::string::String::new();
        heap.stats_print(&mut text).unwrap();
        assert!(!text.is_empty());
        heap.delete();
    }

    #[test]
    fn borrowed_main_heap_delete_is_noop() {
        let heap = Heap::main();
        heap.delete();

        unsafe {
            let ptr = cyo_mimalloc_sys::mi_malloc(64);
            assert!(!ptr.is_null());
            cyo_mimalloc_sys::mi_free(ptr);
        }
    }
}
