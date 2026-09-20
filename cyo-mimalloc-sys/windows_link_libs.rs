// The Windows import libraries the vendored mimalloc sources need.
//
// Included by both `build.rs`, which emits the link directives, and
// `tests/windows_link_libs.rs`, which checks the list against the sources.

/// Import libraries to link when building for a Windows target.
///
/// mimalloc does not declare these itself: the one `#pragma comment(lib, ..)`
/// in `prim/windows/prim.c` sits in the `MI_USE_RTLGENRANDOM` branch we do not
/// compile, and pragmas only work for MSVC anyway. Without them the final link
/// of a binary using this crate fails with unresolved externals.
#[allow(dead_code)]
const WINDOWS_LINK_LIBS: &[&str] = &["advapi32"];

/// Win32 functions the vendored sources call directly, and the import library
/// each one lives in.
///
/// Functions mimalloc resolves with `GetProcAddress` (`BCryptGenRandom` from
/// bcrypt, `GetProcessMemoryInfo` from psapi, the NUMA and large-page entry
/// points) need no import library and are deliberately absent. Everything here
/// is checked against the sources by the test, so an upstream update that
/// starts calling one of these directly fails the test until its library is
/// added to `WINDOWS_LINK_LIBS`.
#[allow(dead_code)]
const WINDOWS_IMPORTS: &[(&str, &str)] = &[
    ("AdjustTokenPrivileges", "advapi32"),
    ("LookupPrivilegeValue", "advapi32"),
    ("LookupPrivilegeValueA", "advapi32"),
    ("LookupPrivilegeValueW", "advapi32"),
    ("OpenProcessToken", "advapi32"),
    ("RegCloseKey", "advapi32"),
    ("RegOpenKeyExA", "advapi32"),
    ("RegQueryValueExA", "advapi32"),
    ("RtlGenRandom", "advapi32"),
    ("SystemFunction036", "advapi32"),
    ("BCryptGenRandom", "bcrypt"),
    ("EnumProcessModules", "psapi"),
    ("GetProcessMemoryInfo", "psapi"),
    ("SHGetKnownFolderPath", "shell32"),
    ("MessageBoxA", "user32"),
    ("MessageBoxW", "user32"),
];

/// The libraries to link for `target_os`, empty everywhere but Windows.
#[allow(dead_code)]
fn windows_link_libs(target_os: &str) -> &'static [&'static str] {
    if target_os == "windows" {
        WINDOWS_LINK_LIBS
    } else {
        &[]
    }
}
