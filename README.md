# rustfs-mimalloc

High-performance [mimalloc](https://github.com/microsoft/mimalloc) V3 global allocator for Rust.

[![Crates.io](https://img.shields.io/crates/v/rustfs-mimalloc.svg)](https://crates.io/crates/rustfs-mimalloc)
[![Documentation](https://docs.rs/rustfs-mimalloc/badge.svg)](https://docs.rs/rustfs-mimalloc)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

## Overview

`rustfs-mimalloc` provides safe, ergonomic Rust bindings to Microsoft's mimalloc V3 memory allocator (v3.5.0). It is designed as a drop-in replacement for the system allocator with excellent multi-threaded performance.

### Key Features

- **V3 only** — exclusively targets mimalloc V3, avoiding multi-version complexity
- **Always aligned** — uses `mi_malloc_aligned` for all allocations, preventing alignment bugs
- **Cross-platform** — supports Linux, macOS, Windows, ARM, RISC-V, and musl
- **Comprehensive API** — stats, options, heap/arena management
- **Production-ready** — addresses [known issues](https://github.com/purpleprotocol/mimalloc_rust/issues) from the reference implementation

## Usage

```toml
[dependencies]
rustfs-mimalloc = "0.1"
```

```rust
use rustfs_mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    // All allocations now use mimalloc
    let v = vec![1, 2, 3, 4, 5];
    println!("{:?}", v);
}
```

## Features

| Feature | Description |
|---------|-------------|
| `secure` | Enable heap encryption (MI_SECURE=4) |
| `debug` | Enable mimalloc debug checks |
| `debug_in_debug` | Auto-enable debug mode in debug builds |
| `override` | Override system malloc/free |
| `local_dynamic_tls` | Use local-dynamic TLS model |
| `no_thp` | Disable Transparent Huge Pages |

Stats, options, version, heap, and arena APIs are always available without a feature flag.

### Profile and Stats API

```rust
use rustfs_mimalloc::MiMalloc;
use rustfs_mimalloc_sys::mi_option_t;

// Get mimalloc version
let version = MiMalloc::version(); // 30500 for V3.5.0

// Get allocation statistics as JSON
let stats = MiMalloc::stats_json();

// Get allocation statistics in mimalloc's text format
let profile = MiMalloc::stats_print();

// Get process memory information in mimalloc's text format
let process_profile = MiMalloc::process_info_print();

// Configure options
MiMalloc::option_set(mi_option_t::mi_option_purge_delay, 0); // Immediate OS memory return

// Get process memory info
let info = MiMalloc::process_info();
println!("Peak RSS: {} bytes", info.peak_rss);
```

### Heap Management

```rust
use rustfs_mimalloc::heap::Heap;

// Create a custom heap
let heap = Heap::new().expect("failed to create heap");

// Allocate from the heap
unsafe {
    let ptr = heap.malloc(128);
    // ... use memory ...
    rustfs_mimalloc_sys::mi_free(ptr as *mut core::ffi::c_void);
}

// Delete heap (moves blocks to main heap)
heap.delete();
```

## Comparison with `mimalloc` crate

| Aspect | `rustfs-mimalloc` | `mimalloc` crate |
|--------|-------------------|-------------------|
| mimalloc version | V3 only (v3.5.0) | V2/V3 (configurable) |
| Alignment handling | Always aligned | Conditional |
| TLS model | Configurable | Forced initial-exec |
| Extended API | Comprehensive | Partial |
| Known issue fixes | All addressed | Various open issues |

## Design Decisions

### V3 Only
This crate exclusively targets mimalloc V3. The V3 branch includes significant improvements:
- Metadata separated from heap objects
- Better arena management
- Guard page support
- Improved multi-threaded performance

### Always Use Aligned Allocation
We always call `mi_malloc_aligned` internally. Previous implementations tried to skip aligned calls for small alignments, which caused [alignment bugs](https://github.com/purpleprotocol/mimalloc_rust/issues/87) and [crashes](https://github.com/purpleprotocol/mimalloc_rust/issues/128).

### No TLS Model Override by Default
The `-ftls-model=initial-exec` flag was [forcing compatibility issues](https://github.com/purpleprotocol/mimalloc_rust/issues/138) with some projects (e.g., polars). Users can opt-in via the `local_dynamic_tls` feature.

## Platform Support

- Linux (x86_64, aarch64, arm, riscv64)
- macOS (x86_64, aarch64)
- Windows (x86_64, aarch64)
- FreeBSD
- musl targets

## MSRV

The minimum supported Rust version is 1.70.0.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

The mimalloc C library is licensed under the MIT License. See [c_src/mimalloc/LICENSE](rustfs-mimalloc-sys/c_src/mimalloc/LICENSE) for details.
