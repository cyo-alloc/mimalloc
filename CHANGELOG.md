# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Changed

- Reduced the feature surface to behavior-changing options only; removed the no-op `extended` feature and fine-grained `secure_level_1..5` aliases.
- Removed the unstable `nightly_allocator_api` feature so the crate remains fully verifiable on stable Rust.
- Kept `secure` as the single heap-encryption feature and mapped it directly to mimalloc's upstream default `MI_SECURE=4`.
- Added `win_direct_tls` as a Windows-only v3 performance opt-in for deployments that can guarantee direct TLS slot availability.
- Consolidated profile output collection into a shared internal FFI helper with preallocated callback storage.
- Changed low-level FFI aliases to use `core::ffi` platform C types.
- Distinguished owned heap handles from borrowed heap handles so `Heap::main()` and `Heap::heap_of()` do not delete heaps they do not own.
- Updated CI and release workflows to test only stable feature combinations that exist.

## [0.1.0] - 2026-08-22

### Added

- Added `rustfs-mimalloc-sys`, a low-level FFI crate that builds and links mimalloc v3.5.0.
- Added `rustfs-mimalloc`, a safe global allocator wrapper for mimalloc v3.
- Added heap and arena management helpers for advanced allocation control.
- Added process memory information APIs through `MiMalloc::process_info`.
- Added memory profile APIs for RustFS profiling handlers:
  - `MiMalloc::stats_json`
  - `MiMalloc::stats_print`
  - `MiMalloc::process_info_print`
  - `MiMalloc::stats_reset`
  - `Heap::stats_json`
  - `Heap::stats_print`
- Added allocator smoke tests, heap tests, stats/profile tests, doctests, and allocation benchmarks.

### Changed

- Stats, options, version, heap, and arena APIs are always available without a feature flag.
