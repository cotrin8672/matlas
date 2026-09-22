use matrust::{
    Complex, Error, ErrorKind, Inputs, MatFile, MatVersion, Matlab, OpenMode, Outputs, Result,
    VariableInfos, Variables, Workspace,
};
use std::{cell::RefCell, ffi::CString};

thread_local! {
    static PERSISTENT: RefCell<Option<matrust::PersistentArray>> = const { RefCell::new(None) };
    static MODULE_LOCK: RefCell<Option<matrust::ModuleLock>> = const { RefCell::new(None) };
}

matrust::mex_entrypoint!(run);

fn run<'mex>(
    cx: &mut Matlab<'mex>,
    inputs: Inputs<'mex>,
    outputs: &mut Outputs<'mex>,
) -> Result<()> {
    if inputs.len() < 2 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "test arguments",
            "command and format required",
        ));
    }
    let command = inputs.get(0).unwrap().scalar()? as u32;
    let format = match inputs.get(1).unwrap().scalar()? as u32 {
        0 => MatVersion::Default,
        1 => MatVersion::V4,
        2 => MatVersion::V6,
        3 => MatVersion::V7,
        4 => MatVersion::V73,
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "format",
                "unknown MAT format",
            ))
        }
    };
    match command {
        1 => {
            let mut file = MatFile::create_with_format(cx, "roundtrip.mat", format)?;
            for (i, value) in inputs.iter().skip(2).enumerate() {
                file.put(&name(i), value)?;
            }
            file.close()?;
        }
        2 => {
            let mut file = MatFile::open(cx, "matlab.mat", OpenMode::Read)?;
            for i in 0..outputs.len() {
                outputs.set(i, file.get(&name(i))?)?;
            }
            file.close()?;
        }
        3 => {
            let mut file = MatFile::open(cx, "roundtrip.mat", OpenMode::Read)?;
            let names = file.variables()?;
            if names.len() != inputs.len() - 2 {
                return Err(Error::new(
                    ErrorKind::Read,
                    "directory",
                    "unexpected variable count",
                ));
            }
            for i in 0..names.len() {
                let info = file.info(&name(i))?;
                if info.numel() != inputs.get(i + 2).unwrap().numel() {
                    return Err(Error::new(
                        ErrorKind::Read,
                        "header",
                        "unexpected element count",
                    ));
                }
            }
            file.close()?;
        }
        4 => {
            let mut file = MatFile::open(cx, "roundtrip.mat", OpenMode::Update)?;
            file.put(&name(0), inputs.get(2).unwrap())?;
            file.delete(&name(1))?;
            file.close()?;
        }
        5 => {
            let mut reader = Variables::open(cx, "roundtrip.mat")?;
            for entry in &mut reader {
                let entry = entry?;
                let i = index(&entry.name);
                outputs.set(i, entry.value)?;
            }
            reader.close()?;
        }
        6 => {
            let mut reader = VariableInfos::open(cx, "roundtrip.mat")?;
            let mut count = 0;
            for entry in &mut reader {
                let _ = entry?;
                count += 1;
            }
            reader.close()?;
            if count != inputs.len() - 2 {
                return Err(Error::new(
                    ErrorKind::Read,
                    "headers",
                    "unexpected variable count",
                ));
            }
        }
        7 => {
            let mut file = MatFile::create(cx, "global.mat")?;
            file.put_global(c"global_value", inputs.get(2).unwrap())?;
            file.close()?;
        }
        8 => {
            let mut file = MatFile::create(cx, "early.mat")?;
            file.put(c"saved", inputs.get(2).unwrap())?;
            file.put(c"", inputs.get(2).unwrap())?;
        }
        9 => {
            let path = "日本語😀/値😀.mat";
            let mut file = MatFile::create_with_format(cx, path, format)?;
            file.put(c"value", inputs.get(2).unwrap())?;
            file.close()?;
            let mut file = MatFile::open(cx, path, OpenMode::Read)?;
            outputs.set(0, file.get(c"value")?)?;
            file.close()?;
        }
        10 => {
            if MatFile::open(cx, "nonexistent.mat", OpenMode::Read).is_ok() {
                return Err(Error::new(
                    ErrorKind::Read,
                    "failure test",
                    "unexpected open success",
                ));
            }
        }
        11 => {
            let value = cx.workspace_get(Workspace::Caller, c"from_caller")?;
            let mut file = MatFile::create(cx, "caller.mat")?;
            file.put(c"from_caller", value.as_ref())?;
            file.close()?;
        }
        12 => {
            let mut file = MatFile::create(cx, "panic.mat")?;
            file.put(c"saved", inputs.get(2).unwrap())?;
            panic!("intentional panic after opening a file");
        }
        13 => {
            return Err(Error::new(
                ErrorKind::Native,
                "test error",
                "invalid: 日本語 %s 100%?tail",
            ))
        }
        14 => {
            let numeric = cx.numeric::<f64>(&[2, 2], &[1.0, 2.0, 3.0, 4.0])?;
            let text = cx.string("日本語")?;
            let logical = cx.logical(&[2, 2], &[true, false, false, true])?;
            let mut cell = cx.cell(&[1, 1])?;
            cell.as_mut().replace_cell(0, Some(cx.scalar(42.0)?))?;
            let sparse = cx.sparse(2, 2, &[0, 1, 2], &[0, 1], &[5.0, 6.0])?;
            outputs.set(0, numeric)?;
            outputs.set(1, text)?;
            outputs.set(2, logical)?;
            outputs.set(3, cell)?;
            outputs.set(4, sparse)?;
        }
        15 => {
            let input = inputs.get(2).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    "sparse access",
                    "one sparse input required",
                )
            })?;
            if input.sparse_data::<f64>()? != [1.0, 2.0] || input.linear_index(&[1, 1])? != 3 {
                return Err(Error::new(
                    ErrorKind::Native,
                    "sparse access",
                    "unexpected sparse contents or indexing",
                ));
            }
            let logical = cx.logical_sparse(2, 2, &[0, 1, 2], &[0, 1], &[true, true])?;
            if logical.as_ref().sparse_logicals()? != [true, true] {
                return Err(Error::new(
                    ErrorKind::Native,
                    "logical sparse access",
                    "unexpected logical sparse contents",
                ));
            }
            let mut numeric = cx.sparse(2, 2, &[0, 1, 2], &[0, 1], &[5.0, 6.0])?;
            numeric.as_mut().sparse_data_mut::<f64>()?[1] = 9.0;
            let mut complex = cx.sparse(
                2,
                2,
                &[0, 1, 2],
                &[0, 1],
                &[
                    Complex {
                        real: 1.0,
                        imag: 2.0,
                    },
                    Complex {
                        real: 3.0,
                        imag: 4.0,
                    },
                ],
            )?;
            complex.as_mut().sparse_data_mut::<Complex<f64>>()?[1].real = 7.0;
            outputs.set(0, logical)?;
            outputs.set(1, numeric)?;
            outputs.set(2, cx.scalar(input.linear_index(&[1, 1])? as f64)?)?;
            outputs.set(3, complex)?;
            outputs.set(4, cx.char_matrix(&["ab", "c"])?)?;
            outputs.set(5, cx.sparse::<f64>(0, 3, &[0, 0, 0, 0], &[], &[])?)?;
            outputs.set(6, cx.logical_sparse(0, 3, &[0, 0, 0, 0], &[], &[])?)?;
        }
        16 => {
            let value = inputs.get(2).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    "persistent store",
                    "one scalar input required",
                )
            })?;
            let persistent = cx.scalar(value.scalar()?)?.persist()?;
            PERSISTENT.with(|slot| *slot.borrow_mut() = Some(persistent));
        }
        17 => {
            let value = PERSISTENT.with(|slot| {
                let slot = slot.borrow();
                let persistent = slot.as_ref().ok_or_else(|| {
                    Error::new(
                        ErrorKind::InvalidInput,
                        "persistent load",
                        "no persistent value",
                    )
                })?;
                cx.duplicate(persistent.as_ref(cx)?)
            })?;
            outputs.set(0, value)?;
        }
        18 => {
            let persistent = PERSISTENT
                .with(|slot| slot.borrow_mut().take())
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::InvalidInput,
                        "persistent remove",
                        "no persistent value",
                    )
                })?;
            persistent.remove(cx)?;
        }
        19 => {
            let lock = cx.lock();
            MODULE_LOCK.with(|slot| *slot.borrow_mut() = Some(lock));
            outputs.set(0, cx.scalar(if cx.is_locked() { 1.0 } else { 0.0 })?)?;
        }
        20 => {
            let lock = MODULE_LOCK
                .with(|slot| slot.borrow_mut().take())
                .ok_or_else(|| {
                    Error::new(ErrorKind::InvalidInput, "module unlock", "no module lock")
                })?;
            lock.release();
            outputs.set(0, cx.scalar(if cx.is_locked() { 1.0 } else { 0.0 })?)?;
        }
        21 => {
            let left = inputs.get(2).ok_or_else(|| {
                Error::new(ErrorKind::InvalidInput, "callback", "left input required")
            })?;
            let right = inputs.get(3).ok_or_else(|| {
                Error::new(ErrorKind::InvalidInput, "callback", "right input required")
            })?;
            let value = cx
                .call(c"plus", &[left, right], 1)?
                .pop()
                .ok_or_else(|| Error::new(ErrorKind::Callback, "callback", "missing output"))?;
            outputs.set(0, value)?;
        }
        22 => {
            let message = cx.string("intentional callback failure")?;
            cx.call(c"error", &[message.as_ref()], 0)?;
        }
        23 => {
            cx.eval("error('matrust:nativeTest','intentional eval failure')")?;
        }
        24 => {
            let value = cx
                .call(c"clock", &[], 1)?
                .pop()
                .ok_or_else(|| Error::new(ErrorKind::Callback, "callback", "missing output"))?;
            outputs.set(0, value)?;
        }
        25 => {
            cx.call(c"drawnow", &[], 0)?;
        }
        26 => {
            let empty = cx.structure(&[1, 1], &[])?;
            let mut value = cx.structure(&[1, 1], &[c"kept", c"removed"])?;
            value
                .as_mut()
                .replace_field(0, c"kept", Some(cx.scalar(42.0)?))?;
            value
                .as_mut()
                .replace_field(0, c"removed", Some(cx.scalar(99.0)?))?;
            let extra = value.as_mut().add_field(c"extra")?;
            value
                .as_mut()
                .replace_field_by_number(0, extra, Some(cx.string("ok")?))?;
            value.as_mut().remove_field(1)?;
            outputs.set(0, empty)?;
            outputs.set(1, value)?;
        }
        27 => {
            let mut value = cx.numeric::<f64>(&[2, 2], &[1.0, 2.0, 3.0, 4.0])?;
            value.reshape(&[1, 4])?;
            value.set_global_flag(true);
            value.set_user_bits(0x5a);
            if !value.as_ref().is_from_global_workspace() || value.as_ref().user_bits() != 0x5a {
                return Err(Error::new(
                    ErrorKind::Native,
                    "owned metadata",
                    "metadata round trip failed",
                ));
            }
            value.make_complex()?;
            if value.as_ref().data::<Complex<f64>>()?[2]
                != (Complex {
                    real: 3.0,
                    imag: 0.0,
                })
            {
                return Err(Error::new(
                    ErrorKind::Native,
                    "make complex",
                    "unexpected converted data",
                ));
            }
            value.make_real()?;
            let mut buffer = cx.calloc(4)?;
            buffer.as_bytes_mut().copy_from_slice(&[1, 2, 3, 4]);
            buffer.resize(8)?;
            if buffer.as_bytes()[..4] != [1, 2, 3, 4] {
                return Err(Error::new(
                    ErrorKind::Native,
                    "MATLAB buffer",
                    "reallocation lost data",
                ));
            }
            outputs.set(0, value)?;
        }
        28 => {
            let scalar = {
                let value = cx.workspace_borrow(Workspace::Caller, c"from_caller_scalar")?;
                value.as_ref().scalar()?
            };
            let value = cx.scalar(scalar + 1.0)?;
            cx.workspace_put(Workspace::Caller, c"from_rust", value.as_ref())?;
            outputs.set(0, value)?;
        }
        29 => {
            let input = inputs.get(2).ok_or_else(|| {
                Error::new(ErrorKind::InvalidInput, "object", "object input required")
            })?;
            if cx.property(input, 0, c"Value")?.as_ref().scalar()? != 11.0 {
                return Err(Error::new(
                    ErrorKind::Native,
                    "get borrowed property",
                    "unexpected original value",
                ));
            }
            let mut object = cx.duplicate(input)?;
            if object.property(0, c"Value")?.as_ref().scalar()? != 11.0 {
                return Err(Error::new(
                    ErrorKind::Native,
                    "get property",
                    "unexpected original value",
                ));
            }
            let replacement = cx.scalar(22.0)?;
            object
                .as_mut()
                .set_property(0, c"Value", replacement.as_ref())?;
            outputs.set(0, object.property(0, c"Value")?)?;
        }
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "command",
                "unknown command",
            ))
        }
    }
    if !outputs.is_empty()
        && !matches!(
            command,
            2 | 5 | 9 | 14 | 15 | 17 | 19 | 20 | 21 | 24 | 26 | 27 | 28 | 29
        )
    {
        outputs.set(0, cx.scalar(0.0)?)?;
    }
    Ok(())
}
fn name(i: usize) -> CString {
    CString::new(format!("v{i}")).unwrap()
}
fn index(name: &std::ffi::CStr) -> usize {
    name.to_str()
        .unwrap()
        .strip_prefix('v')
        .unwrap()
        .parse()
        .unwrap()
}
