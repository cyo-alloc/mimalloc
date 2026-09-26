//! Checks that the build script links the import library of every Win32
//! function that the vendored sources call directly.
//!
//! A missing library does not break the build of this crate, because the static
//! library still compiles. Instead, the final link of a binary that uses the
//! crate fails on Windows with unresolved externals. These tests read the
//! sources instead of linking, so they find the problem on any host.

use std::fs;
use std::path::{Path, PathBuf};

mod common;

use common::{calls, strip_c_comments_and_strings};

include!("../windows_link_libs.rs");

fn windows_prim_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/mimalloc/src/prim/windows")
}

/// Returns the Windows platform sources of mimalloc, without comments and
/// string literals.
///
/// A name that the sources only pass to `GetProcAddress` is in a string
/// literal, so it does not count as a call.
fn windows_sources() -> String {
    let dir = windows_prim_dir();
    let mut out = String::new();
    for entry in fs::read_dir(&dir).expect("mimalloc submodule is not checked out") {
        let path = entry.unwrap().path();
        if matches!(path.extension().and_then(|e| e.to_str()), Some("c" | "h")) {
            out.push_str(&strip_c_comments_and_strings(
                &fs::read_to_string(&path).unwrap(),
            ));
            out.push('\n');
        }
    }
    assert!(
        !out.trim().is_empty(),
        "no sources found in {}",
        dir.display()
    );
    out
}

#[test]
fn every_win32_import_has_its_library_linked() {
    let sources = windows_sources();
    let linked = windows_link_libs("windows");

    for (symbol, lib) in WINDOWS_IMPORTS {
        if calls(&sources, symbol) {
            assert!(
                linked.contains(lib),
                "the vendored sources call {symbol}, which lives in {lib}.lib, but the build \
                 script does not link {lib}; add it to WINDOWS_LINK_LIBS",
            );
        }
    }
}

/// Checks that the scan still finds three calls that mimalloc has made since v1.
///
/// If upstream moves or renames the sources, or the scan stops recognising
/// calls, the test above finds no calls and passes without checking anything.
#[test]
fn the_large_page_imports_are_still_found() {
    let sources = windows_sources();
    for symbol in [
        "OpenProcessToken",
        "LookupPrivilegeValue",
        "AdjustTokenPrivileges",
    ] {
        assert!(
            calls(&sources, symbol),
            "{symbol} is no longer found as a call in the Windows sources; the scan in this \
             test must be updated",
        );
    }
}

/// Checks that a name that mimalloc only reaches through `GetProcAddress` does
/// not count as a call.
#[test]
fn dynamically_loaded_names_are_not_counted_as_calls() {
    let sources = windows_sources();
    assert!(!calls(&sources, "BCryptGenRandom"));
    assert!(!calls(&sources, "GetProcessMemoryInfo"));
}

#[test]
fn no_libraries_are_linked_off_windows() {
    assert!(windows_link_libs("linux").is_empty());
    assert!(windows_link_libs("macos").is_empty());
    assert!(!windows_link_libs("windows").is_empty());
}
