# cyo-mimalloc

## Configuration without Cargo features

We don't use cargo features, instead using build environment variables, so we don't have to think too much about all the features but can just pass things on to mimalloc as much as possible. You can set these in your project's `.cargo/config.toml`, for example:

```toml
[env]
MI_SECURE = "4"
MI_DEFAULT_ALLOW_THP = "0"
```

## License

The Rust code is licensed under the [Apache License 2.0](https://github.com/cyo-alloc/mimalloc/blob/master/LICENSE).
The mimalloc sources bundled in `cyo-mimalloc-sys` are [MIT licensed](https://github.com/microsoft/mimalloc/blob/main/LICENSE).

## Attributions

This is a fork of [rustfs-mimalloc](https://github.com/houseme/rustfs-mimalloc) by houseme, also licensed under Apache-2.0, with various changes. rustfs-mimalloc in turn drew on [mimalloc_rust](https://github.com/purpleprotocol/mimalloc_rust) by Octavian Oncescu (the `mimalloc` and `libmimalloc-sys` crates), and on the problems reported against it.
Its MIT license is included as [LICENSE-mimalloc_rust](https://github.com/cyo-alloc/mimalloc/blob/master/LICENSE-mimalloc_rust).

## LLM disclaimer

This project is AI-assisted. All work after the fork was fully AI-assisted. I believe rustfs-mimalloc was also AI-assisted, although the original project it was based on was not. Of course almost all of the code that actually runs is just plain mimalloc, which is of course extremely mature and trusted software. Some care was taken for the API docs to be nice to read and not contain too much slop.
