use core::ffi::{CStr, c_char, c_void};
use core::fmt;

/// Writes a string that mimalloc allocated to `out`, then frees it.
///
/// Returns an error if `ptr` is null, which means that mimalloc could not
/// produce the string.
///
/// # Safety
///
/// `ptr` must be null, or a NUL-terminated string that mimalloc allocated and
/// that the caller owns.
pub(crate) unsafe fn write_owned_c_string(
    out: &mut (impl fmt::Write + ?Sized),
    ptr: *mut c_char,
) -> fmt::Result {
    if ptr.is_null() {
        return Err(fmt::Error);
    }
    // SAFETY: the caller guarantees that `ptr` is a NUL-terminated string.
    let result = write_bytes(out, unsafe { CStr::from_ptr(ptr) }.to_bytes());
    // SAFETY: the caller owns the string, and nothing reads it after this.
    unsafe { cyo_mimalloc_sys::mi_free(ptr as *mut c_void) };
    result
}

/// Runs `print` with an output callback and an argument that forward all
/// output to `out`.
///
/// The callback and the argument stay valid until `print` returns.
pub(crate) fn write_output(
    out: &mut dyn fmt::Write,
    print: impl FnOnce(Option<cyo_mimalloc_sys::mi_output_fun>, *mut c_void),
) -> fmt::Result {
    let mut sink = Sink {
        out,
        result: Ok(()),
    };
    print(Some(output_callback), &mut sink as *mut Sink as *mut c_void);
    sink.result
}

struct Sink<'a> {
    out: &'a mut dyn fmt::Write,
    result: fmt::Result,
}

unsafe extern "C" fn output_callback(msg: *const c_char, arg: *mut c_void) {
    if msg.is_null() || arg.is_null() {
        return;
    }
    // SAFETY: `write_output` passes a pointer to a `Sink` that lives until
    // `print` returns, and mimalloc only calls this callback before then.
    let sink = unsafe { &mut *(arg as *mut Sink) };
    if sink.result.is_ok() {
        // SAFETY: mimalloc passes a NUL-terminated string in `msg`.
        sink.result = write_bytes(sink.out, unsafe { CStr::from_ptr(msg) }.to_bytes());
    }
}

/// Writes `bytes` as text, and replaces invalid UTF-8 with U+FFFD.
fn write_bytes(out: &mut (impl fmt::Write + ?Sized), bytes: &[u8]) -> fmt::Result {
    for chunk in bytes.utf8_chunks() {
        out.write_str(chunk.valid())?;
        if !chunk.invalid().is_empty() {
            out.write_char(char::REPLACEMENT_CHARACTER)?;
        }
    }
    Ok(())
}
