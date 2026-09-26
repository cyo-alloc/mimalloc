//! Build script for cyo-mimalloc-sys.
//!
//! Compiles mimalloc V3 into a static library with the `cc` crate. The build
//! environment configures it, as the `cyo-mimalloc` crate documentation
//! describes.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;

include!("windows_link_libs.rs");

struct TargetCfg {
    triple: String,
    arch: String,
    os: String,
    env: String,
}

impl TargetCfg {
    fn from_env() -> Self {
        Self {
            triple: env::var("TARGET").unwrap_or_default(),
            arch: env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default(),
            os: env::var("CARGO_CFG_TARGET_OS").unwrap_or_default(),
            env: env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default(),
        }
    }

    fn needs_armv6_atomic(&self) -> bool {
        self.arch == "arm"
            && (self.triple.starts_with("armv6") || self.triple.starts_with("arm-unknown"))
    }
}

fn main() {
    let target = TargetCfg::from_env();
    let is_debug = env::var("PROFILE").as_deref() == Ok("debug");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=windows_link_libs.rs");
    println!("cargo:rerun-if-changed=c_src/mimalloc");

    // docs.rs builds every target from a Linux host, where no MSVC compiler
    // exists. rustdoc does not link, so the library is not needed there.
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    if env::var_os("DOCS_RS").is_some() {
        return;
    }

    let mut build = cc::Build::new();
    build.file("c_src/mimalloc/src/static.c");
    build.include("c_src/mimalloc/include");
    build.include("c_src/mimalloc/src");

    if !is_debug {
        build.opt_level(3);
        build.define("NDEBUG", None);
        build.define("MI_BUILD_RELEASE", None);
    } else {
        build.opt_level(0);
        build.debug(true);
    }

    // Build configuration comes from the environment of the final build, never
    // from Cargo features, so no dependency can change it for the application.
    let debug = build_env_level("MI_DEBUG", 3).unwrap_or(0);
    build.define("MI_DEBUG", debug.to_string().as_str());
    if let Some(secure) = build_env_level("MI_SECURE", 4) {
        build.define("MI_SECURE", secure.to_string().as_str());
    }
    if build_env_level("MI_NO_THP", 1) == Some(1) {
        build.define("MI_NO_THP", "1");
    }

    // TLS model: initial-exec is fastest, but a shared library loaded with
    // `dlopen` needs local-dynamic. The value is still checked when building
    // for MSVC, which has no equivalent flag and would only warn about it.
    println!("cargo:rerun-if-env-changed=CYO_MIMALLOC_TLS_MODEL");
    let tls_model = env::var("CYO_MIMALLOC_TLS_MODEL").unwrap_or_else(|_| "initial-exec".into());
    match tls_model.as_str() {
        "initial-exec" | "local-dynamic" | "global-dynamic" | "local-exec" => {
            if target.env != "msvc" {
                build.flag(format!("-ftls-model={tls_model}"));
            }
        }
        other => panic!(
            "CYO_MIMALLOC_TLS_MODEL={other:?} is not one of initial-exec, local-dynamic, \
             global-dynamic or local-exec"
        ),
    }

    // The `MI_DEFAULT_*` variables set option defaults. The `MIMALLOC_*`
    // environment variables still override them at run time.
    define_option_defaults(&mut build);

    build.flag_if_supported("-Wno-unused-function");
    build.flag_if_supported("-Wno-unused-parameter");

    // ARM: no `-march` is passed, so the compiler uses the target's default
    // architecture level. Forcing ARMv8.1-A would break older cores such as
    // the Raspberry Pi 4's Cortex-A72.

    build.compile("mimalloc");

    // ARMv6 has no native 64-bit atomics, so they come from libatomic.
    if target.needs_armv6_atomic() {
        println!("cargo:rustc-link-lib=atomic");
    }

    // Windows: the sources call into advapi32 to enable large pages, and
    // mimalloc declares none of the import libraries it needs. See
    // windows_link_libs.rs.
    for lib in windows_link_libs(&target.os) {
        println!("cargo:rustc-link-lib={lib}");
    }

    // Dependent crates find the headers through `DEP_MIMALLOC_INCLUDE`.
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:include={manifest_dir}/c_src/mimalloc/include");
}

/// Reads an integer from 0 to `max` from the build environment variable
/// `name`.
///
/// Returns `None` if the variable is not set, and panics if its value is out
/// of range.
fn build_env_level(name: &str, max: u8) -> Option<u8> {
    println!("cargo:rerun-if-env-changed={name}");
    let value = env::var(name).ok()?;
    match value.trim().parse::<u8>() {
        Ok(level) if level <= max => Some(level),
        _ => panic!("{name}={value:?} must be an integer from 0 to {max}"),
    }
}

/// Passes each `MI_DEFAULT_*` variable of the build environment to the C
/// compiler as a macro.
///
/// Only the macros that the vendored sources define with `#ifndef` are
/// passed. The function warns about every other `MI_DEFAULT_*` variable.
fn define_option_defaults(build: &mut cc::Build) {
    let mut known = BTreeSet::new();
    collect_default_macros(Path::new("c_src/mimalloc/src"), &mut known);
    collect_default_macros(Path::new("c_src/mimalloc/include"), &mut known);

    for name in &known {
        println!("cargo:rerun-if-env-changed={name}");
        if let Ok(value) = env::var(name) {
            let value = value.trim();
            if value.is_empty() {
                panic!("{name} is set but empty");
            }
            build.define(name, value);
        }
    }

    for (name, _) in env::vars() {
        if name.starts_with("MI_DEFAULT_") && !known.contains(&name) {
            println!("cargo:warning={name} is not an option default mimalloc accepts; ignoring it");
        }
    }
}

fn collect_default_macros(dir: &Path, out: &mut BTreeSet<String>) {
    for entry in fs::read_dir(dir).expect("failed to read mimalloc sources") {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_default_macros(&path, out);
            continue;
        }
        if !matches!(path.extension().and_then(|e| e.to_str()), Some("c" | "h")) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            if let Some(name) = line.trim().strip_prefix("#ifndef ")
                && name.starts_with("MI_DEFAULT_")
            {
                out.insert(name.trim().to_owned());
            }
        }
    }
}
