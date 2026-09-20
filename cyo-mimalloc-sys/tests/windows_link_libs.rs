//! The Windows import libraries the build script links must cover every Win32
//! function the vendored sources call directly.
//!
//! Missing one does not break the build of this crate — the static library
//! compiles fine — but the final link of any binary using it fails on Windows
//! with unresolved externals. These tests read the sources instead of linking,
//! so they catch that from any host.

use std::fs;
use std::path::{Path, PathBuf};

include!("../windows_link_libs.rs");

fn windows_prim_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/mimalloc/src/prim/windows")
}

/// The Windows primitive sources, with comments and string literals removed so
/// that a name only passed to `GetProcAddress` does not read as a call.
fn windows_sources() -> String {
    let dir = windows_prim_dir();
    let mut out = String::new();
    for entry in fs::read_dir(&dir).expect("mimalloc submodule is not checked out") {
        let path = entry.unwrap().path();
        if matches!(path.extension().and_then(|e| e.to_str()), Some("c" | "h")) {
            out.push_str(&strip_comments_and_strings(
                &fs::read_to_string(&path).unwrap(),
            ));
            out.push('\n');
        }
    }
    assert!(!out.trim().is_empty(), "no sources found in {}", dir.display());
    out
}

fn strip_comments_and_strings(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i < bytes.len() && !(bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/')) {
                    i += 1;
                }
                i = (i + 2).min(bytes.len());
                out.push(' ');
            }
            quote @ (b'"' | b'\'') => {
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
                out.push(' ');
            }
            byte => {
                out.push(byte as char);
                i += 1;
            }
        }
    }
    out
}

/// Whether `symbol` is called (or declared) in `text`, rather than appearing as
/// part of a longer identifier such as the `PGetProcessMemoryInfo` typedef.
fn calls(text: &str, symbol: &str) -> bool {
    text.match_indices(symbol).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + symbol.len()..].trim_start().chars().next();
        !before.is_some_and(|c| c.is_alphanumeric() || c == '_') && after == Some('(')
    })
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

/// Guards the test above against going vacuous: if upstream moves or renames
/// the sources, or the scan stops recognising calls, the loop would quietly
/// find nothing and pass. mimalloc has called these three since V1.
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
            "{symbol} no longer reads as a call in the Windows sources; the scan in this test \
             has gone stale and no longer proves anything",
        );
    }
}

/// A name only reached through `GetProcAddress` is not an import.
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
