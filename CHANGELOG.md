# Changelog

All notable changes to this project will be documented in this file.

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

- Kept the `extended` feature as a backward-compatible no-op because stats, options, version, heap, and arena APIs are now always available.
