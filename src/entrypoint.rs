//! Implementation details for the C-owned MEX entry boundary.
pub use rustmex::mxArray;
use rustmex::{Lhs, MxArray, Rhs};
use std::{
    ffi::c_char,
    panic::{catch_unwind, AssertUnwindSafe},
};

extern "C" {
    pub fn rustmat_mex_anchor();
}

/// # Safety
/// Called only by mex_entry.c with MATLAB's valid input/output arrays and
/// writable nonempty error buffers. Counts and capacities must be correct.
#[allow(clippy::too_many_arguments)]
pub unsafe fn dispatch(
    handler: fn(Lhs<'_>, Rhs<'_, '_>) -> rustmex::Result<()>,
    nlhs: i32,
    plhs: *mut *mut mxArray,
    nrhs: i32,
    prhs: *const *const mxArray,
    id: *mut c_char,
    id_len: usize,
    message: *mut c_char,
    message_len: usize,
) -> i32 {
    // SAFETY: these two stack buffers are supplied exclusively by mex_entry.c.
    let id = unsafe { std::slice::from_raw_parts_mut(id.cast::<u8>(), id_len) };
    let message = unsafe { std::slice::from_raw_parts_mut(message.cast::<u8>(), message_len) };
    let result = catch_unwind(AssertUnwindSafe(|| {
        assert!(nlhs >= 0 && nrhs >= 0);
        // MATLAB is allowed to pass NULL for an empty input array.
        let inputs = if nrhs == 0 {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(prhs, nrhs as usize) }
        };
        let rhs: Vec<&mxArray> = inputs
            .iter()
            .map(|&p| {
                assert!(!p.is_null());
                // SAFETY: MATLAB owns these full input arrays for this invocation.
                unsafe { &*p }
            })
            .collect();
        let mut lhs: Vec<Option<MxArray>> = (0..nlhs).map(|_| None).collect();
        match handler(&mut lhs, &rhs) {
            Ok(()) => {
                for (i, value) in lhs.into_iter().enumerate() {
                    if let Some(value) = value {
                        // SAFETY: a successful invocation transfers each owned
                        // output exactly once into MATLAB's writable output slots.
                        unsafe {
                            plhs.add(i)
                                .write(MxArray::transfer_responsibility_ptr(value));
                        }
                    }
                }
                0
            }
            Err(error) => {
                let error_id = error.id();
                copy_text(
                    id,
                    if valid_id(error_id, id.len()) {
                        error_id
                    } else {
                        "rustmat:mex:error"
                    },
                );
                copy_text(message, &error.to_string());
                // lhs and every temporary are dropped before control returns to C.
                1
            }
        }
    }));
    match result {
        Ok(status) => status,
        Err(payload) => {
            copy_text(id, "rustmat:mex:panic");
            let text = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("Rust panic in MEX handler");
            copy_text(message, text);
            // Arbitrary panic payloads may panic again in Drop. Normal string
            // payloads are safe to destroy; only unknown custom payloads leak.
            if payload.is::<String>() || payload.is::<&str>() {
                drop(payload);
            } else {
                std::mem::forget(payload);
            }
            1
        }
    }
}

fn valid_id(id: &str, capacity: usize) -> bool {
    id.len() < capacity
        && id.contains(':')
        && id.split(':').all(|part| {
            let mut bytes = part.bytes();
            bytes.next().is_some_and(|b| b.is_ascii_alphabetic())
                && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
}

fn copy_text(buffer: &mut [u8], text: &str) {
    let mut end = text.len().min(buffer.len() - 1);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    for (out, byte) in buffer[..end].iter_mut().zip(text.bytes()) {
        *out = if byte == 0 { b'?' } else { byte };
    }
    buffer[end] = 0;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn error_buffers_preserve_utf8_and_reject_invalid_ids() {
        let mut buffer = [255; 5];
        copy_text(&mut buffer, "a😀b");
        assert_eq!(&buffer[..2], b"a\0");
        copy_text(&mut buffer, "a\0b");
        assert_eq!(&buffer[..4], b"a?b\0");
        for id in ["", "plain", "a:", "a:1x", "a:b%", "a:b\0"] {
            assert!(!valid_id(id, 256));
        }
        assert!(valid_id("rustmat:input:invalid", 256));
        assert!(!valid_id("a:b", 3));
    }
}
