# rustfs-mimalloc

[![Crates.io](https://img.shields.io/crates/v/rustfs-mimalloc.svg)](https://crates.io/crates/rustfs-mimalloc)
[![Documentation](https://docs.rs/rustfs-mimalloc/badge.svg)](https://docs.rs/rustfs-mimalloc)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](../LICENSE)

High-performance [mimalloc](https://github.com/microsoft/mimalloc) V3 global allocator for Rust.

## Quick Start

```toml
[dependencies]
rustfs-mimalloc = "0.5.2"
```

```rust
use rustfs_mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    let v = vec![1, 2, 3, 4, 5];
    println!("{:?}", v);
}
```

## Features

| Feature | Description |
|---------|-------------|
| `secure` | Heap allocation encryption (MI_SECURE=4) |
| `debug` | mimalloc debug checks |
| `debug_in_debug` | Auto-enable `debug` in Cargo debug builds |
| `override` | Override system `malloc`/`free` |
| `local_dynamic_tls` | Use local-dynamic TLS model |
| `no_thp` | Disable Transparent Huge Pages |

## API

### Allocator

`MiMalloc` implements `GlobalAlloc` — all methods use `mi_malloc_aligned` for guaranteed alignment.

### Stats & Options

```rust
use rustfs_mimalloc::MiMalloc;
use rustfs_mimalloc_sys::mi_option_t;

let version = MiMalloc::version();       // 30501 = V3.5.1
let json    = MiMalloc::stats_json();    // stats as JSON
let text    = MiMalloc::stats_print();   // stats as text
let info    = MiMalloc::process_info();  // ProcessInfo struct

MiMalloc::option_set(mi_option_t::mi_option_purge_delay, 0);
```

### Small Free Fast Paths

`MiMalloc::free_csize`, `free_csize_nonnull`, `free_small`, and
`free_small_nonnull` expose mimalloc V3.5.1's small and constant-size free fast
paths. These APIs are unsafe: the pointer must come from mimalloc, and the
caller must preserve the original allocation size contract.

### Threadpool Hint

```rust
rustfs_mimalloc::set_current_thread_in_threadpool();
```

Call this from custom thread-pool worker threads to set mimalloc's current-thread threadpool marker.

### Heap & Arena

```rust
use rustfs_mimalloc::heap::Heap;

let heap = Heap::new().unwrap();
unsafe {
    let ptr = heap.malloc(128);
    rustfs_mimalloc_sys::mi_free(ptr as *mut core::ffi::c_void);
}
heap.delete();
```

## MSRV

Rust 1.96.0. Rolling support window for the latest three stable Rust releases.

## License

Apache-2.0.
