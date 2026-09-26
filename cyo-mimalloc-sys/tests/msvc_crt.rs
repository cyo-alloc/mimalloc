//! Checks that the build script leaves the choice of the MSVC C runtime to
//! Cargo.
//!
//! With `static_crt` unset, `cc` passes `/MT` when the target feature
//! `crt-static` is on, and `/MD` when it is off. That is how
//! `-C target-feature=+crt-static` produces a binary that needs neither
//! `vcruntime140.dll` nor `ucrtbase.dll`. If the build script chose the flag,
//! every build would use one runtime, and mixing the two runtimes in one binary
//! is a link error. Only a Windows build of a dependent program would show the
//! error, so this test reads the build script instead of compiling.

use std::fs;
use std::path::Path;

mod common;

use common::strip_rust_comments;

#[test]
fn the_build_script_does_not_pin_the_msvc_crt() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("build.rs");
    let source = strip_rust_comments(&fs::read_to_string(&path).unwrap());

    for needle in ["static_crt", "/MT", "-MT", "/MD", "-MD"] {
        assert!(
            !source.contains(needle),
            "build.rs mentions {needle}, which pins the MSVC C runtime; leave the choice to \
             `cc`, which follows the crt-static target feature",
        );
    }
}

/// The helper has to keep string literals, or the test above would pass on a
/// build script that does pin the runtime.
#[test]
fn stripping_rust_comments_keeps_string_literals() {
    let source = strip_rust_comments(
        r#"
        // build.flag("-MT");
        /* build.flag("-MD"); */
        build.flag("-MT"); // a real one
        let url = "https://example.invalid"; // not a comment
        "#,
    );

    assert!(source.contains(r#"build.flag("-MT")"#));
    assert!(!source.contains("-MD"));
    assert!(source.contains("https://example.invalid"));
    assert!(!source.contains("a real one"));
}
