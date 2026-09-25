//! The only unsafe boundary in the public MEX entrypoint.
use crate::{runtime, ArrayRef, Error, Inputs, Matlab, Outputs, Result};
use std::{
    ffi::c_char,
    panic::{catch_unwind, AssertUnwindSafe},
};

pub use crate::ffi::RawArray;

extern "C" {
    pub fn matrust_mex_anchor();
}

pub type Handler = for<'mex> fn(&mut Matlab<'mex>, Inputs<'mex>, &mut Outputs<'mex>) -> Result<()>;

/// Called only by `mex_entry.c` with MATLAB-owned input/output pointers.
#[allow(clippy::too_many_arguments)]
pub unsafe fn dispatch<const INPUTS: usize, const OUTPUTS: usize>(
    handler: for<'mex> fn(
        &mut Matlab<'mex>,
        Inputs<'mex, INPUTS>,
        &mut Outputs<'mex, OUTPUTS>,
    ) -> Result<()>,
    nlhs: i32,
    plhs: *mut *mut crate::ffi::RawArray,
    nrhs: i32,
    prhs: *const *const crate::ffi::RawArray,
    id: *mut c_char,
    id_len: usize,
    message: *mut c_char,
    message_len: usize,
) -> i32 {
    let id = unsafe { std::slice::from_raw_parts_mut(id.cast::<u8>(), id_len) };
    let message = unsafe { std::slice::from_raw_parts_mut(message.cast::<u8>(), message_len) };
    let result = catch_unwind(AssertUnwindSafe(|| {
        if nlhs < 0 || nrhs < 0 || (nlhs > 0 && plhs.is_null()) || (nrhs > 0 && prhs.is_null()) {
            return Err(Error::new(
                crate::ErrorKind::InvalidInput,
                "MEX entrypoint",
                "invalid MATLAB counts or pointers",
            ));
        }
        let nlhs = usize::try_from(nlhs).map_err(|_| {
            Error::new(
                crate::ErrorKind::InvalidInput,
                "MEX entrypoint",
                "negative output count",
            )
        })?;
        let nrhs = usize::try_from(nrhs).map_err(|_| {
            Error::new(
                crate::ErrorKind::InvalidInput,
                "MEX entrypoint",
                "negative input count",
            )
        })?;
        for index in 0..nlhs {
            unsafe { plhs.add(index).write(std::ptr::null_mut()) };
        }
        if INPUTS != usize::MAX && nrhs != INPUTS {
            return Err(Error::new(
                crate::ErrorKind::InvalidInput,
                "MEX inputs",
                format!("expected {INPUTS} inputs, got {nrhs}"),
            ));
        }
        if OUTPUTS != usize::MAX && nlhs != OUTPUTS {
            return Err(Error::new(
                crate::ErrorKind::InvalidInput,
                "MEX outputs",
                format!("expected {OUTPUTS} outputs, got {nlhs}"),
            ));
        }
        let _invocation = runtime::begin_invocation()?;
        let mut context = Matlab::new();
        let mut values = Vec::with_capacity(nrhs);
        for index in 0..nrhs {
            let raw = unsafe { *prhs.add(index) };
            let raw = std::ptr::NonNull::new(raw.cast_mut()).ok_or_else(|| {
                Error::new(
                    crate::ErrorKind::InvalidInput,
                    "MEX input",
                    "null input array",
                )
            })?;
            values.push(unsafe { ArrayRef::from_raw(raw) });
        }
        let inputs = Inputs::new(values);
        let mut outputs = Outputs::new(nlhs);
        handler(&mut context, inputs, &mut outputs)?;
        for index in 0..nlhs {
            if let Some(value) = outputs.take(index)? {
                unsafe { plhs.add(index).write(value.into_raw()) };
            }
        }
        Ok::<(), Error>(())
    }));
    match result {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => {
            copy_text(id, error.id());
            copy_text(message, &error.to_string());
            1
        }
        Err(payload) => {
            copy_text(id, "matlas:panic");
            let text = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("Rust panic in MEX handler");
            copy_text(message, text);
            let _ = catch_unwind(AssertUnwindSafe(|| drop(payload)));
            1
        }
    }
}

fn copy_text(buffer: &mut [u8], text: &str) {
    if buffer.is_empty() {
        return;
    }
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
    fn preserves_utf8_boundaries() {
        let mut buffer = [255; 5];
        copy_text(&mut buffer, "a😀b");
        assert_eq!(&buffer[..2], b"a\0");
        copy_text(&mut buffer, "a\0b");
        assert_eq!(&buffer[..4], b"a?b\0");
    }
}
