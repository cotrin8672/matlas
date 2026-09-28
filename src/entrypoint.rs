//! The only unsafe boundary in the public MEX entrypoint.
use crate::{runtime, ArrayRef, DynInputs, Error, Inputs, Matlab, Outputs, Result};
use std::{
    ffi::c_char,
    panic::{catch_unwind, AssertUnwindSafe},
};

pub use crate::ffi::RawArray;

extern "C" {
    pub fn matrust_mex_anchor();
}

pub type Handler =
    for<'mex> fn(&mut Matlab<'mex>, DynInputs<'mex>, &mut Outputs<'mex>) -> Result<()>;

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
    unsafe {
        dispatch_inner(
            Some(INPUTS),
            |context, values, outputs| {
                let values = values.try_into().unwrap_or_else(|_| unreachable!());
                handler(context, Inputs::new(values), outputs)
            },
            nlhs,
            plhs,
            nrhs,
            prhs,
            id,
            id_len,
            message,
            message_len,
        )
    }
}

/// Called only by `mex_entry.c` for a handler with variable input arity.
#[allow(clippy::too_many_arguments)]
pub unsafe fn dispatch_dynamic<const OUTPUTS: usize>(
    handler: for<'mex> fn(
        &mut Matlab<'mex>,
        DynInputs<'mex>,
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
    unsafe {
        dispatch_inner(
            None,
            |context, values, outputs| handler(context, DynInputs::new(values), outputs),
            nlhs,
            plhs,
            nrhs,
            prhs,
            id,
            id_len,
            message,
            message_len,
        )
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn dispatch_inner<const OUTPUTS: usize>(
    expected_inputs: Option<usize>,
    handler: impl for<'mex> FnOnce(
        &mut Matlab<'mex>,
        Vec<ArrayRef<'mex>>,
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
        if let Some(expected) = expected_inputs {
            if nrhs != expected {
                return Err(Error::new(
                    crate::ErrorKind::InvalidInput,
                    "MEX inputs",
                    format!("expected {expected} inputs, got {nrhs}"),
                ));
            }
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
        let mut outputs = Outputs::new(nlhs);
        handler(&mut context, values, &mut outputs)?;
        outputs.validate()?;
        let first_set = outputs.values[0].is_some();
        if nlhs == 0 && first_set && plhs.is_null() {
            return Err(Error::new(
                crate::ErrorKind::InvalidInput,
                "MEX entrypoint",
                "null first output pointer",
            ));
        }
        let count = nlhs.max(usize::from(first_set));
        for index in 0..count {
            let value = outputs.values[index].take().expect("validated output");
            unsafe { plhs.add(index).write(value.into_raw()) };
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
