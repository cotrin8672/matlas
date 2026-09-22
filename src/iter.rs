use crate::{ffi, ArrayInfo, Error, ErrorKind, MatFile, Matlab, OpenMode, Result};
use rustmex::MxArray;
use std::{
    ffi::{CStr, CString},
    iter::FusedIterator,
    path::Path,
    ptr::NonNull,
};

/// A file variable's owned name and its array or metadata.
pub struct Named<T> {
    pub name: CString,
    pub value: T,
}

struct Cursor<'matlab> {
    file: MatFile<'matlab>,
    remaining: usize,
    done: bool,
}
impl<'matlab> Cursor<'matlab> {
    fn open(context: &'matlab Matlab, path: &Path) -> Result<Self> {
        // Query the directory on a DIFFERENT handle. No other MAT operations may
        // touch the sequential handle between open and matGetNext* calls.
        let mut directory = MatFile::open(context, path, OpenMode::Read)?;
        let remaining = directory.variables()?.len();
        directory.close()?;
        let file = MatFile::open(context, path, OpenMode::Read)?;
        Ok(Self {
            file,
            remaining,
            done: false,
        })
    }
    fn next(&mut self, info: bool) -> Option<Result<Named<ffi::Array>>> {
        if self.done || self.remaining == 0 {
            self.done = true;
            return None;
        }
        let (mut name, mut code) = (std::ptr::null(), 0);
        // SAFETY: dedicated sequential handle; the next API is never mixed with
        // random access. Names are copied before the following native operation.
        let raw =
            unsafe { ffi::rustmat_next(self.file.ptr(), &mut name, i32::from(info), &mut code) };
        let Some(raw) = NonNull::new(raw) else {
            self.done = true;
            return Some(Err(Error::native(
                ErrorKind::UnexpectedEnd,
                "next",
                "could not read all variables reported by the directory",
                -1,
                code,
            )));
        };
        let value = ffi::Array(raw);
        if name.is_null() {
            self.done = true;
            return Some(Err(Error::new(
                ErrorKind::Native,
                "next",
                "native variable name was null",
            )));
        }
        let name = unsafe { CStr::from_ptr(name) }.to_owned();
        self.remaining -= 1;
        Some(Ok(Named { name, value }))
    }
}

/// A dedicated sequential full-array reader. The input file must not be modified
/// concurrently. A premature native NULL is an error, never silently EOF.
pub struct Variables<'matlab> {
    cursor: Cursor<'matlab>,
}
impl<'matlab> Variables<'matlab> {
    pub fn open(context: &'matlab Matlab, path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            cursor: Cursor::open(context, path.as_ref())?,
        })
    }
    pub fn close(self) -> Result<()> {
        self.cursor.file.close()
    }
}
impl Iterator for Variables<'_> {
    type Item = Result<Named<MxArray>>;
    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.next(false).map(|result| {
            result.map(|Named { name, value }| {
                let raw = value.0.as_ptr();
                std::mem::forget(value);
                // SAFETY: transfer the single owner of a full matGetNextVariable array.
                let value = unsafe { MxArray::assume_responsibility_ptr(raw) };
                Named { name, value }
            })
        })
    }
}
impl FusedIterator for Variables<'_> {}

/// A dedicated sequential header reader. Headers cannot become normal arrays.
pub struct VariableInfos<'matlab> {
    cursor: Cursor<'matlab>,
}
impl<'matlab> VariableInfos<'matlab> {
    pub fn open(context: &'matlab Matlab, path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            cursor: Cursor::open(context, path.as_ref())?,
        })
    }
    pub fn close(self) -> Result<()> {
        self.cursor.file.close()
    }
}
impl<'matlab> Iterator for VariableInfos<'matlab> {
    type Item = Result<Named<ArrayInfo<'matlab>>>;
    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.next(true).map(|result| {
            result.map(|Named { name, value }| Named {
                name,
                value: ArrayInfo::new(value, self.cursor.file.context),
            })
        })
    }
}
impl FusedIterator for VariableInfos<'_> {}
