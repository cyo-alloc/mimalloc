// The Windows import libraries the vendored mimalloc sources need.
//
// Included by both `build.rs`, which emits the link directives, and
// `tests/windows_link_libs.rs`, which checks the list against the sources.

/// The import libraries to link when building for a Windows target.
///
/// mimalloc does not declare these itself. Its one `#pragma comment(lib, ..)`,
/// in `prim/windows/prim.c`, is in the `MI_USE_RTLGENRANDOM` branch, which
/// this crate does not compile. Only MSVC reads such a pragma in any case.
/// Without these libraries, the final link of a binary that uses this crate
/// fails with unresolved externals.
#[allow(dead_code)]
const WINDOWS_LINK_LIBS: &[&str] = &["advapi32"];

/// Win32 functions that the vendored sources may call, each with the import
/// library that contains it.
///
/// A function that mimalloc resolves with `GetProcAddress` needs no import
/// library, so `WINDOWS_LINK_LIBS` leaves its library out. Examples are
/// `BCryptGenRandom` from bcrypt, `GetProcessMemoryInfo` from psapi, and some
/// NUMA and large-page functions. The test checks each entry against the
/// sources. If an upstream update starts to call one of these functions
/// directly, the test fails until you add its library to `WINDOWS_LINK_LIBS`.
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

/// Returns the import libraries to link for `target_os`. The list is empty for
/// every target except Windows.
#[allow(dead_code)]
fn windows_link_libs(target_os: &str) -> &'static [&'static str] {
    if target_os == "windows" {
        WINDOWS_LINK_LIBS
    } else {
        &[]
    }
}
