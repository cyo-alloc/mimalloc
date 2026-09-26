//! Separate heaps and arenas.
//!
//! mimalloc keeps the blocks of each heap on pages of their own. To allocate
//! a group of blocks apart from the rest of the program:
//!
//! 1. Create a heap with [`Heap::new`]. To give the heap memory of its own,
//!    first reserve an exclusive arena with [`reserve_os_memory`], then create
//!    the heap with [`Heap::new_in_arena`].
//! 2. Allocate from the heap with [`Heap::malloc`] or another allocation
//!    method.
//! 3. Free each block with [`free`], on any thread.
//! 4. Drop the heap. mimalloc moves the blocks that are still allocated to the
//!    main heap. To free them all at once instead, call [`Heap::destroy`].
//!
//! # Examples
//!
//! ```rust
//! use cyo_mimalloc::heap::Heap;
//!
//! let heap = Heap::new().expect("mimalloc could not create a heap");
//! let block = heap.malloc(64);
//! assert!(!block.is_null());
//! unsafe {
//!     block.write_bytes(0, 64);
//!     cyo_mimalloc::heap::free(block);
//! }
//! drop(heap);
//! ```

use core::ffi::c_void;
use core::fmt;
use core::ptr::NonNull;

// ── Error type ──────────────────────────────────────────────────────────────

/// The error that the arena functions return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArenaError {
    /// mimalloc could not reserve or register the memory.
    ///
    /// The OS may be out of memory, or an argument may be invalid.
    Failed,
}

impl fmt::Display for ArenaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed => f.write_str("mimalloc could not reserve or register the arena memory"),
        }
    }
}

impl core::error::Error for ArenaError {}

// ── Heap ────────────────────────────────────────────────────────────────────

/// A handle to a mimalloc heap.
///
/// You can allocate from a heap and free its blocks on any thread. Dropping a
/// heap that [`Heap::new`] or [`Heap::new_in_arena`] created deletes it, as
/// [`Heap::delete`] does. Dropping the handle that [`Heap::main`] returns
/// leaves the main heap in place.
pub struct Heap {
    ptr: NonNull<cyo_mimalloc_sys::mi_heap_t>,
    owned: bool,
}

// A `Heap` that does not own its heap only ever refers to the main heap, which
// mimalloc never deletes. A borrowed handle to any other heap could outlive
// it, and its safe methods would then use freed memory.

// SAFETY: mimalloc v3 heaps can be used from any thread at once, as the heap
// section of `mimalloc.h` states. The handle holds nothing that is tied to
// the thread that created it.
unsafe impl Send for Heap {}
// SAFETY: as for `Send`. The methods that take `&self` only call mimalloc
// functions that are safe to call on one heap from several threads.
unsafe impl Sync for Heap {}

impl Heap {
    /// Creates a heap.
    ///
    /// Returns `None` if mimalloc cannot allocate the heap.
    pub fn new() -> Option<Self> {
        // SAFETY: `mi_heap_new` has no preconditions.
        NonNull::new(unsafe { cyo_mimalloc_sys::mi_heap_new() }).map(Self::owned)
    }

    /// Creates a heap that allocates only from the arena `arena_id`.
    ///
    /// Returns `None` if mimalloc cannot allocate the heap.
    pub fn new_in_arena(arena_id: ArenaId) -> Option<Self> {
        // SAFETY: an `ArenaId` only comes from mimalloc, and mimalloc never
        // removes an arena.
        NonNull::new(unsafe { cyo_mimalloc_sys::mi_heap_new_in_arena(arena_id.0) }).map(Self::owned)
    }

    /// Returns a handle to the main heap, which the global allocator uses.
    ///
    /// Dropping the handle leaves the main heap in place.
    pub fn main() -> Self {
        // SAFETY: `mi_heap_main` has no preconditions.
        let ptr = unsafe { cyo_mimalloc_sys::mi_heap_main() };
        Self::borrowed(NonNull::new(ptr).expect("mi_heap_main returned null"))
    }

    /// Returns the identity of this heap, to compare with [`heap_of`].
    pub fn id(&self) -> HeapId {
        HeapId(self.ptr.as_ptr() as usize)
    }

    /// Returns whether this heap allocated the block at `ptr`.
    ///
    /// # Safety
    ///
    /// `ptr` must point into a block that mimalloc allocated, and the block
    /// must stay allocated during the call.
    pub unsafe fn contains(&self, ptr: *const u8) -> bool {
        // SAFETY: `self.ptr` is a live heap, and the caller guarantees that
        // `ptr` is in a live mimalloc block, so its page stays in place.
        unsafe { cyo_mimalloc_sys::mi_heap_contains(self.ptr.as_ptr(), ptr as *const c_void) }
    }

    /// Allocates `size` bytes from this heap.
    ///
    /// Returns a null pointer if mimalloc cannot allocate the memory. Free the
    /// block with [`free`], on any thread.
    pub fn malloc(&self, size: usize) -> *mut u8 {
        // SAFETY: `self.ptr` is a live heap.
        unsafe { cyo_mimalloc_sys::mi_heap_malloc(self.ptr.as_ptr(), size) as *mut u8 }
    }

    /// Allocates `size` zeroed bytes from this heap.
    ///
    /// Returns a null pointer if mimalloc cannot allocate the memory. Free the
    /// block with [`free`], on any thread.
    pub fn zalloc(&self, size: usize) -> *mut u8 {
        // SAFETY: `self.ptr` is a live heap.
        unsafe { cyo_mimalloc_sys::mi_heap_zalloc(self.ptr.as_ptr(), size) as *mut u8 }
    }

    /// Allocates `size` bytes aligned to `alignment` from this heap.
    ///
    /// Returns a null pointer if `alignment` is not a power of two, or if
    /// mimalloc cannot allocate the memory. Free the block with [`free`], on
    /// any thread.
    pub fn malloc_aligned(&self, size: usize, alignment: usize) -> *mut u8 {
        // SAFETY: `self.ptr` is a live heap.
        unsafe {
            cyo_mimalloc_sys::mi_heap_malloc_aligned(self.ptr.as_ptr(), size, alignment) as *mut u8
        }
    }

    /// Resizes the block at `ptr` to `new_size` bytes.
    ///
    /// If mimalloc has to move the block, it allocates the new block from this
    /// heap.
    /// Returns the new block, or a null pointer if mimalloc cannot allocate
    /// the memory. On failure, the block at `ptr` stays allocated.
    ///
    /// # Safety
    ///
    /// `ptr` must be null or a block that mimalloc allocated and that you have
    /// not freed. After a successful call, you must use only the returned
    /// pointer, and free it with [`free`].
    pub unsafe fn realloc(&self, ptr: *mut u8, new_size: usize) -> *mut u8 {
        // SAFETY: `self.ptr` is a live heap, and the caller guarantees that
        // `ptr` is null or a live mimalloc block.
        unsafe {
            cyo_mimalloc_sys::mi_heap_realloc(self.ptr.as_ptr(), ptr as *mut c_void, new_size)
                as *mut u8
        }
    }

    /// Deletes this heap. Dropping the heap does the same.
    ///
    /// mimalloc moves the blocks that are still allocated to the main heap,
    /// so they stay valid. For the handle that [`Heap::main`] returns, this
    /// function does nothing.
    pub fn delete(self) {
        if self.owned {
            // SAFETY: this handle owns the heap, and it is consumed here, so
            // nothing uses the heap afterwards.
            unsafe { cyo_mimalloc_sys::mi_heap_delete(self.ptr.as_ptr()) }
        }
        core::mem::forget(self);
    }

    /// Destroys this heap and frees every block that is still allocated from
    /// it.
    ///
    /// For the handle that [`Heap::main`] returns, this function does nothing.
    ///
    /// # Safety
    ///
    /// You must not use a block from this heap after the call.
    pub unsafe fn destroy(self) {
        if self.owned {
            // SAFETY: this handle owns the heap and is consumed here. The
            // caller guarantees that no block from the heap is used again.
            unsafe { cyo_mimalloc_sys::mi_heap_destroy(self.ptr.as_ptr()) };
        }
        core::mem::forget(self);
    }

    /// Returns memory that this heap no longer uses on the current thread.
    ///
    /// With `force`, mimalloc also purges unused memory immediately, instead
    /// of after `mi_option_purge_delay`.
    pub fn collect(&self, force: bool) {
        // SAFETY: `self.ptr` is a live heap.
        unsafe { cyo_mimalloc_sys::mi_heap_collect(self.ptr.as_ptr(), force) }
    }

    /// Writes this heap's allocation statistics to `out` as JSON.
    ///
    /// # Errors
    ///
    /// Returns [`fmt::Error`] if mimalloc cannot produce the statistics, or if
    /// `out` returns an error.
    pub fn stats_json(&self, out: &mut (impl fmt::Write + ?Sized)) -> fmt::Result {
        // SAFETY: `self.ptr` is a live heap. With a null buffer,
        // `mi_heap_stats_get_json` returns a string that mimalloc allocated
        // and that the caller owns, or null.
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

    /// Writes this heap's allocation statistics to `out` as mimalloc's text
    /// table.
    ///
    /// # Errors
    ///
    /// Returns [`fmt::Error`] if `out` returns an error.
    pub fn stats_print(&self, out: &mut dyn fmt::Write) -> fmt::Result {
        // SAFETY: `self.ptr` is a live heap, and `write_output` passes a
        // callback and an argument that stay valid for the call.
        crate::ffi::write_output(out, |out, arg| unsafe {
            cyo_mimalloc_sys::mi_heap_stats_print_out(self.ptr.as_ptr(), out, arg);
        })
    }

    /// Returns the heap's `mi_heap_t` pointer, for the `cyo-mimalloc-sys`
    /// functions.
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
            // SAFETY: this handle owns the heap and is being dropped, so
            // nothing uses the heap afterwards.
            unsafe { cyo_mimalloc_sys::mi_heap_delete(self.ptr.as_ptr()) }
        }
    }
}

/// The identity of a heap.
///
/// Compare it with [`Heap::id`] to find out which heap allocated a block. The
/// value stays comparable after the heap is deleted, but mimalloc can then
/// give the same identity to a new heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeapId(usize);

/// Returns the identity of the heap that allocated the block at `ptr`.
///
/// Returns `None` if no mimalloc heap allocated `ptr`. A block that outlived
/// a deleted heap belongs to the main heap.
///
/// # Safety
///
/// `ptr` must point into a block that mimalloc allocated, and the block must
/// stay allocated during the call.
pub unsafe fn heap_of(ptr: *const u8) -> Option<HeapId> {
    // SAFETY: the caller guarantees that `ptr` is in a live mimalloc block, so
    // its page stays in place during the lookup.
    let heap = unsafe { cyo_mimalloc_sys::mi_heap_of(ptr as *const c_void) };
    (!heap.is_null()).then_some(HeapId(heap as usize))
}

/// Frees the block at `ptr`, which any mimalloc heap may have allocated.
///
/// You can free a block on any thread. Nothing happens if `ptr` is null.
///
/// # Safety
///
/// `ptr` must be null or a block that mimalloc allocated and that you own.
/// You must not use the block after the call.
#[inline]
pub unsafe fn free(ptr: *mut u8) {
    // SAFETY: the caller guarantees that `ptr` is null or a mimalloc block
    // that it owns and no longer uses.
    unsafe { cyo_mimalloc_sys::mi_free(ptr as *mut c_void) }
}

// ── Arena ───────────────────────────────────────────────────────────────────

/// The identifier of an arena, for [`Heap::new_in_arena`].
#[derive(Debug, Clone, Copy)]
pub struct ArenaId(cyo_mimalloc_sys::mi_arena_id_t);

// SAFETY: an arena id is a value that mimalloc looks up. mimalloc accepts it
// on any thread.
unsafe impl Send for ArenaId {}
// SAFETY: as for `Send`.
unsafe impl Sync for ArenaId {}

/// Reserves `size` bytes of OS memory as an arena, and returns its id.
///
/// This function does at run time what the `reserve_os_memory` option does at
/// startup. The arguments work as follows:
///
/// - `commit` commits the memory immediately.
/// - `allow_large` lets mimalloc use large OS pages for the arena.
/// - `exclusive` keeps the arena for the heaps that [`Heap::new_in_arena`]
///   creates for it. Other allocations do not use it.
///
/// # Errors
///
/// Returns [`ArenaError::Failed`] if mimalloc cannot reserve the memory.
pub fn reserve_os_memory(
    size: usize,
    commit: bool,
    allow_large: bool,
    exclusive: bool,
) -> Result<ArenaId, ArenaError> {
    let mut id = core::ptr::null_mut();
    // SAFETY: `id` is a valid place for mimalloc to write the arena id to.
    let rc = unsafe {
        cyo_mimalloc_sys::mi_reserve_os_memory_ex(size, commit, allow_large, exclusive, &mut id)
    };
    if rc == 0 {
        Ok(ArenaId(id))
    } else {
        Err(ArenaError::Failed)
    }
}

/// Reserves `pages` huge OS pages of 1 GiB each, spread over `numa_nodes` NUMA
/// nodes.
///
/// A `numa_nodes` of 0 uses all nodes. mimalloc stops trying after
/// `timeout_msecs` milliseconds. This function does at run time what the
/// `reserve_huge_os_pages` option does at startup. The kernel must have 1 GiB
/// huge pages available.
///
/// # Errors
///
/// Returns [`ArenaError::Failed`] if mimalloc cannot reserve the pages.
pub fn reserve_huge_os_pages_interleave(
    pages: usize,
    numa_nodes: usize,
    timeout_msecs: usize,
) -> Result<(), ArenaError> {
    // SAFETY: `mi_reserve_huge_os_pages_interleave` has no preconditions.
    let rc = unsafe {
        cyo_mimalloc_sys::mi_reserve_huge_os_pages_interleave(pages, numa_nodes, timeout_msecs)
    };
    if rc == 0 {
        Ok(())
    } else {
        Err(ArenaError::Failed)
    }
}

/// Reserves `pages` huge OS pages of 1 GiB each on the NUMA node `numa_node`.
///
/// mimalloc stops trying after `timeout_msecs` milliseconds. This function
/// does at run time what the `reserve_huge_os_pages` and
/// `reserve_huge_os_pages_at` options do at startup.
///
/// # Errors
///
/// Returns [`ArenaError::Failed`] if mimalloc cannot reserve the pages.
pub fn reserve_huge_os_pages_at(
    pages: usize,
    numa_node: i32,
    timeout_msecs: usize,
) -> Result<(), ArenaError> {
    // SAFETY: `mi_reserve_huge_os_pages_at` has no preconditions.
    let rc =
        unsafe { cyo_mimalloc_sys::mi_reserve_huge_os_pages_at(pages, numa_node, timeout_msecs) };
    if rc == 0 {
        Ok(())
    } else {
        Err(ArenaError::Failed)
    }
}

/// Gives the `size` bytes at `start` to mimalloc as an arena, and returns its
/// id.
///
/// mimalloc aligns `start` up to [`arena_min_alignment`]. The other arguments
/// describe the memory:
///
/// - `is_committed`: the memory is already committed.
/// - `is_pinned`: mimalloc must not decommit or reset the memory, for example
///   because it consists of large OS pages.
/// - `is_zero`: the memory is zeroed.
/// - `numa_node`: the NUMA node of the memory, or -1 if it has none.
/// - `exclusive`: the arena is kept for the heaps that [`Heap::new_in_arena`]
///   creates for it.
///
/// # Errors
///
/// Returns [`ArenaError::Failed`] if the memory is too small after alignment,
/// or if mimalloc cannot register it.
///
/// # Safety
///
/// `start` must point to `size` bytes of readable and writable memory. The
/// memory must stay valid for the rest of the process, because mimalloc never
/// gives an arena back. You must not use the memory yourself.
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
    // SAFETY: the caller guarantees that the memory is valid and unused for
    // the rest of the process. `id` is a valid place for the arena id.
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

/// Returns the alignment, in bytes, of the start of an arena.
#[inline]
pub fn arena_min_alignment() -> usize {
    // SAFETY: `mi_arena_min_alignment` has no preconditions.
    unsafe { cyo_mimalloc_sys::mi_arena_min_alignment() }
}

/// Returns the smallest size of an arena, in bytes.
#[inline]
pub fn arena_min_size() -> usize {
    // SAFETY: `mi_arena_min_size` has no preconditions.
    unsafe { cyo_mimalloc_sys::mi_arena_min_size() }
}

/// Returns the size, in bytes, of the largest object that mimalloc allocates
/// from an arena.
///
/// mimalloc allocates a larger object directly from the OS. The
/// `arena_max_object_size` option sets this size.
#[inline]
pub fn arena_max_object_size() -> usize {
    // SAFETY: `mi_arena_max_object_size` has no preconditions.
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
        let ptr = heap.malloc(128);
        assert!(!ptr.is_null());
        unsafe {
            core::ptr::write_bytes(ptr, 0xCD, 128);
            free(ptr);
        }
        heap.delete();
    }

    #[test]
    fn heap_aligned_alloc() {
        let heap = Heap::new().unwrap();
        for pow in 0..=12 {
            let align = 1usize << pow;
            let ptr = heap.malloc_aligned(64, align);
            assert!(!ptr.is_null());
            assert_eq!(ptr as usize % align, 0, "align={align}");
            unsafe { free(ptr) };
        }
        assert!(heap.malloc_aligned(64, 3).is_null());
        heap.delete();
    }

    #[test]
    fn heap_of_identifies_the_allocating_heap() {
        let a = Heap::new().unwrap();
        let b = Heap::new().unwrap();
        let block = a.malloc(64);
        unsafe {
            assert_eq!(heap_of(block), Some(a.id()));
            assert_ne!(heap_of(block), Some(b.id()));
            assert!(a.contains(block));
            assert!(!b.contains(block));
        }
        unsafe { free(block) };
    }

    /// A block that outlives its heap moves to the main heap. `heap_of` used
    /// to return a `Heap` handle here, whose safe methods then used the
    /// freed heap and crashed the process.
    #[test]
    fn a_block_that_outlives_its_heap_belongs_to_the_main_heap() {
        let heap = Heap::new().unwrap();
        let block = heap.malloc(64);
        drop(heap);
        unsafe {
            assert_eq!(heap_of(block), Some(Heap::main().id()));
            free(block);
        }
    }

    #[test]
    fn free_accepts_null() {
        unsafe { free(core::ptr::null_mut()) };
    }

    #[test]
    fn arena_error_describes_itself() {
        use std::string::ToString;
        let error: &dyn core::error::Error = &ArenaError::Failed;
        assert_eq!(
            error.to_string(),
            "mimalloc could not reserve or register the arena memory"
        );
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
