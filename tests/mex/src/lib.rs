use matrust::{
    Error, ErrorKind, Inputs, MatFile, MatVersion, Matlab, OpenMode, Outputs, Result,
    VariableInfos, Variables, Workspace,
};
use std::ffi::CString;

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
            let mut file = MatFile::create_with_format(cx, "unicode.mat", format)?;
            file.put(c"value", inputs.get(2).unwrap())?;
            file.close()?;
            let mut file = MatFile::open(cx, "unicode.mat", OpenMode::Read)?;
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
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "command",
                "unknown command",
            ))
        }
    }
    if !outputs.is_empty() && !matches!(command, 2 | 5 | 9 | 14) {
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
