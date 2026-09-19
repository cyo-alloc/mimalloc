use core::ffi::{CStr, c_char, c_void};
use core::fmt;

/// Write a string that mimalloc allocated to `out`, then free it.
///
/// A null `ptr` means mimalloc could not produce the string.
///
/// # Safety
/// `ptr` must be null or a NUL-terminated string allocated by mimalloc that the
/// caller owns.
pub(crate) unsafe fn write_owned_c_string(
    out: &mut (impl fmt::Write + ?Sized),
    ptr: *mut c_char,
) -> fmt::Result {
    if ptr.is_null() {
        return Err(fmt::Error);
    }
    let result = write_bytes(out, unsafe { CStr::from_ptr(ptr) }.to_bytes());
    unsafe { cyo_mimalloc_sys::mi_free(ptr as *mut c_void) };
    result
}

/// Run `print` with an output callback that forwards everything to `out`.
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
    let sink = unsafe { &mut *(arg as *mut Sink) };
    if sink.result.is_ok() {
        sink.result = write_bytes(sink.out, unsafe { CStr::from_ptr(msg) }.to_bytes());
    }
}

/// Write `bytes` as text, replacing invalid UTF-8 with U+FFFD.
fn write_bytes(out: &mut (impl fmt::Write + ?Sized), bytes: &[u8]) -> fmt::Result {
    for chunk in bytes.utf8_chunks() {
        out.write_str(chunk.valid())?;
        if !chunk.invalid().is_empty() {
            out.write_char(char::REPLACEMENT_CHARACTER)?;
        }
    }
    Ok(())
}
