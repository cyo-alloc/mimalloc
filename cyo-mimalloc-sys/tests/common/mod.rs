//! Helpers shared by the tests that read source files.

/// Remove comments and string literals from C source, so that a name only
/// passed to `GetProcAddress` does not read as a call.
#[allow(dead_code)]
pub fn strip_c_comments_and_strings(text: &str) -> String {
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

/// Remove comments from Rust source, keeping string literals: the compiler
/// flags the build script passes are strings. Single quotes are left alone,
/// because in Rust they mostly start a lifetime, not a literal.
#[allow(dead_code)]
pub fn strip_rust_comments(text: &str) -> String {
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
            b'"' => {
                // Kept, but consumed here so that a `//` inside it is not read
                // as the start of a comment.
                out.push('"');
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    let escaped = bytes[i] == b'\\';
                    out.push(bytes[i] as char);
                    i += 1;
                    if escaped && i < bytes.len() {
                        out.push(bytes[i] as char);
                        i += 1;
                    }
                }
                out.push('"');
                i += 1;
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
#[allow(dead_code)]
pub fn calls(text: &str, symbol: &str) -> bool {
    text.match_indices(symbol).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + symbol.len()..].trim_start().chars().next();
        !before.is_some_and(|c| c.is_alphanumeric() || c == '_') && after == Some('(')
    })
}
