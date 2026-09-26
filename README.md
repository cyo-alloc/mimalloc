# cyo-mimalloc

[mimalloc](https://github.com/microsoft/mimalloc) V3 as a Rust global
allocator.

```toml
[dependencies]
cyo-mimalloc = "0.0.1"
```

```rust
use cyo_mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
```

The repository holds two crates:

- [`cyo-mimalloc`](https://docs.rs/cyo-mimalloc): the global allocator, plus
  safe wrappers for heaps, arenas, statistics and runtime options. It is
  `no_std` and does not use `alloc`.
- [`cyo-mimalloc-sys`](https://docs.rs/cyo-mimalloc-sys): the raw FFI
  bindings. It compiles the bundled mimalloc sources with the `cc` crate.

Both are tested on Linux and on Windows (MSVC, with the dynamic and the
static C runtime).

## Configuration without Cargo features

Cargo merges features across the whole dependency tree, so a feature would
let any library change the allocator for the entire program. These crates
have no features. mimalloc is configured instead through environment
variables of the build that produces the final binary, usually set in that
project's `.cargo/config.toml`:

```toml
[env]
MI_SECURE = "4"
MI_DEFAULT_ALLOW_THP = "0"
```

| Variable | Effect |
|---|---|
| `MI_SECURE` | Secure mode, 0 to 4. |
| `MI_DEBUG` | Internal assertions and consistency checks, 0 to 3. |
| `MI_NO_THP` | 1 compiles out requests for transparent huge pages. |
| `CYO_MIMALLOC_TLS_MODEL` | `local-dynamic` for a shared library loaded with `dlopen`. |
| `MI_DEFAULT_<NAME>` | The default value of a runtime option, for the options that accept one. |

The runtime options can also be set through `MIMALLOC_<NAME>` variables when
the program starts, or from code. The
[crate documentation](https://docs.rs/cyo-mimalloc) describes every variable
and option, and when each one takes effect.

## Updates

A scheduled workflow opens a pull request when mimalloc publishes a new V3
release.

## Origin

This is a fork of [rustfs-mimalloc](https://github.com/houseme/rustfs-mimalloc)
by houseme.

## License

The Rust code is licensed under the
[Apache License 2.0](https://github.com/cyo-alloc/mimalloc/blob/main/LICENSE).
The mimalloc sources bundled in `cyo-mimalloc-sys` are
[MIT licensed](https://github.com/microsoft/mimalloc/blob/main/LICENSE).
