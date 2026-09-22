use crate::{ffi, Error, ErrorKind, Result};
use std::{cell::RefCell, panic::catch_unwind, ptr::NonNull};

struct Entry {
    raw: NonNull<ffi::RawArray>,
    generation: u64,
}

#[derive(Default)]
struct Runtime {
    invocation_depth: usize,
    active_external_borrows: usize,
    deferred_destroy: Vec<NonNull<ffi::RawArray>>,
    persistent: Vec<Option<Entry>>,
    next_generation: u64,
    exit_registered: bool,
}

thread_local! {
    static RUNTIME: RefCell<Runtime> = RefCell::new(Runtime::default());
}

pub(crate) struct InvocationGuard;

pub(crate) fn begin_invocation() -> Result<InvocationGuard> {
    RUNTIME.with(|runtime| {
        let mut runtime = runtime.borrow_mut();
        if runtime.active_external_borrows != 0 {
            return Err(Error::new(
                ErrorKind::Busy,
                "enter MEX function",
                "a nested invocation attempted to run while MATLAB-owned data was borrowed",
            ));
        }
        runtime.invocation_depth += 1;
        Ok(InvocationGuard)
    })
}

impl Drop for InvocationGuard {
    fn drop(&mut self) {
        let pending = RUNTIME.with(|runtime| {
            let mut runtime = runtime.borrow_mut();
            runtime.invocation_depth -= 1;
            if runtime.invocation_depth == 0 && runtime.active_external_borrows == 0 {
                Some(runtime.deferred_destroy.drain(..).collect::<Vec<_>>())
            } else {
                None
            }
        });
        if let Some(pending) = pending {
            destroy_all(pending);
        }
    }
}

pub(crate) struct ExternalBorrowGuard;

pub(crate) fn begin_external_borrow() -> ExternalBorrowGuard {
    RUNTIME.with(|runtime| runtime.borrow_mut().active_external_borrows += 1);
    ExternalBorrowGuard
}

impl Drop for ExternalBorrowGuard {
    fn drop(&mut self) {
        let pending = RUNTIME.with(|runtime| {
            let mut runtime = runtime.borrow_mut();
            runtime.active_external_borrows -= 1;
            if runtime.active_external_borrows == 0 {
                Some(runtime.deferred_destroy.drain(..).collect::<Vec<_>>())
            } else {
                None
            }
        });
        if let Some(pending) = pending {
            destroy_all(pending);
        }
    }
}

pub(crate) fn require_unborrowed(operation: &'static str) -> Result<()> {
    RUNTIME.with(|runtime| {
        if runtime.borrow().active_external_borrows == 0 {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::Busy,
                operation,
                "MATLAB-owned data is currently borrowed",
            ))
        }
    })
}

pub(crate) unsafe fn destroy_or_defer(raw: NonNull<ffi::RawArray>) {
    let destroy_now = RUNTIME.with(|runtime| {
        let mut runtime = runtime.borrow_mut();
        if runtime.active_external_borrows == 0 {
            true
        } else {
            runtime.deferred_destroy.push(raw);
            false
        }
    });
    if destroy_now {
        unsafe { ffi::matrust_array_destroy(raw.as_ptr()) };
    }
}

fn destroy_all(values: impl IntoIterator<Item = NonNull<ffi::RawArray>>) {
    for value in values {
        unsafe { ffi::matrust_array_destroy(value.as_ptr()) };
    }
}

pub(crate) fn persist(raw: NonNull<ffi::RawArray>) -> Result<(usize, u64)> {
    require_unborrowed("persist array")?;
    RUNTIME.with(|runtime| {
        let mut runtime = runtime.borrow_mut();
        if !runtime.exit_registered {
            let status = unsafe { ffi::matrust_at_exit(at_exit) };
            if status != 0 {
                return Err(Error::native_status("register MEX cleanup", status));
            }
            runtime.exit_registered = true;
        }
        unsafe { ffi::matrust_make_array_persistent(raw.as_ptr()) };
        runtime.next_generation = runtime.next_generation.wrapping_add(1).max(1);
        let generation = runtime.next_generation;
        if let Some((slot, vacant)) = runtime
            .persistent
            .iter_mut()
            .enumerate()
            .find(|(_, value)| value.is_none())
        {
            *vacant = Some(Entry { raw, generation });
            Ok((slot, generation))
        } else {
            let slot = runtime.persistent.len();
            runtime.persistent.push(Some(Entry { raw, generation }));
            Ok((slot, generation))
        }
    })
}

pub(crate) fn persistent(slot: usize, generation: u64) -> Result<NonNull<ffi::RawArray>> {
    RUNTIME.with(|runtime| {
        runtime
            .borrow()
            .persistent
            .get(slot)
            .and_then(Option::as_ref)
            .filter(|entry| entry.generation == generation)
            .map(|entry| entry.raw)
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "persistent array", "stale key"))
    })
}

pub(crate) fn remove_persistent(slot: usize, generation: u64) -> Result<()> {
    require_unborrowed("remove persistent array")?;
    let raw = RUNTIME.with(|runtime| {
        let mut runtime = runtime.borrow_mut();
        let current = runtime
            .persistent
            .get(slot)
            .and_then(Option::as_ref)
            .filter(|entry| entry.generation == generation)
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "persistent array", "stale key"))?;
        let _ = current;
        let entry = runtime.persistent[slot]
            .take()
            .expect("checked persistent entry");
        Ok::<_, Error>(entry.raw)
    })?;
    unsafe { ffi::matrust_array_destroy(raw.as_ptr()) };
    Ok(())
}

unsafe extern "C" fn at_exit() {
    let _ = catch_unwind(|| {
        let pending = RUNTIME.with(|runtime| {
            let mut runtime = runtime.borrow_mut();
            let persistent = runtime
                .persistent
                .drain(..)
                .filter_map(|entry| entry.map(|entry| entry.raw));
            let mut pending: Vec<_> = persistent.collect();
            pending.append(&mut runtime.deferred_destroy);
            runtime.exit_registered = false;
            pending
        });
        destroy_all(pending);
    });
}

pub(crate) fn module_id() -> usize {
    at_exit as usize
}
