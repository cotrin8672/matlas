use rustmat::{
    ArrayInfo, ErrorKind, MatFile, MatVersion, Matlab, OpenMode, VariableInfos, Variables,
};
use rustmex::convert::{FromMatlab, ToMatlab};
use rustmex::prelude::*;
use std::ffi::CString;

rustmat::mex_entrypoint!(run);
fn run(lhs: Lhs, rhs: Rhs) -> rustmex::Result<()> {
    rustmex::assert!(
        rhs.len() >= 2,
        "rustmat:test:args",
        "command and format required"
    );
    let command = f64::from_matlab(rhs[0])? as u32;
    let format = match f64::from_matlab(rhs[1])? as u32 {
        0 => MatVersion::Default,
        1 => MatVersion::V4,
        2 => MatVersion::V6,
        3 => MatVersion::V7,
        4 => MatVersion::V73,
        _ => panic!("bad test format"),
    };
    // SAFETY: this entrypoint is executing synchronously on MATLAB's calling
    // thread. All handles/headers are local; only full arrays go into lhs.
    let context = unsafe { Matlab::attach() };
    match command {
        1 => {
            let mut file = MatFile::create_with_format(&context, "roundtrip.mat", format)?;
            for (i, value) in rhs[2..].iter().enumerate() {
                file.put(&name(i), value)?;
            }
            assert!(matches!(file.get(c"v0"), Err(e) if e.kind == ErrorKind::InvalidMode));
            file.close()?;
        }
        2 => {
            let mut file = MatFile::open(&context, "matlab.mat", OpenMode::Read)?;
            for (i, out) in lhs.iter_mut().enumerate() {
                *out = Some(file.get(&name(i))?);
            }
            file.close()?; // outputs must survive this close
            return Ok(());
        }
        3 => {
            let mut file = MatFile::open(&context, "roundtrip.mat", OpenMode::Read)?;
            let names = file.variables()?;
            assert_eq!(names.len(), rhs.len() - 2);
            for (i, value) in rhs[2..].iter().enumerate() {
                assert!(names.contains(&name(i)));
                let info = file.info(&name(i))?;
                check_info(&info, value);
                assert!(!info.is_global());
            }
            let stream = file.stream();
            if format != MatVersion::V73 {
                assert!(stream.is_some());
            }
            if let Some(mut stream) = stream {
                assert!(!stream.has_error());
                let _ = stream.is_eof();
                let _ = stream.position()?;
                stream.clear_error();
                assert!(!stream.has_error());
            }
            assert!(matches!(file.get(c"absent"), Err(e) if e.kind == ErrorKind::Read));
            println!("missing variable native code: {:?}", file.last_error());
            // A failure must not poison a subsequent successful read.
            let value = file.get(c"v0")?;
            assert_eq!(value.dimensions(), rhs[2].dimensions());
            assert!(matches!(file.put(c"x", rhs[2]), Err(e) if e.kind == ErrorKind::InvalidMode));
            assert!(matches!(file.delete(c"x"), Err(e) if e.kind == ErrorKind::InvalidMode));
            file.close()?;
        }
        4 => {
            let mut file = MatFile::open(&context, "roundtrip.mat", OpenMode::Update)?;
            file.put(c"v0", rhs[2])?;
            file.delete(c"v1")?;
            assert!(file.delete(c"absent").is_err());
            assert!(file.get(c"v1").is_err());
            file.close()?;
        }
        5 => {
            let mut reader = Variables::open(&context, "roundtrip.mat")?;
            let mut seen = vec![false; lhs.len()];
            for entry in &mut reader {
                let entry = entry?;
                let i = index(&entry.name);
                assert!(!seen[i]);
                seen[i] = true;
                lhs[i] = Some(entry.value);
            }
            assert!(seen.iter().all(|seen| *seen));
            assert!(reader.next().is_none());
            assert!(reader.next().is_none());
            reader.close()?;
            return Ok(());
        }
        6 => {
            let mut reader = VariableInfos::open(&context, "roundtrip.mat")?;
            let mut seen = vec![false; rhs.len() - 2];
            for entry in &mut reader {
                let entry = entry?;
                let i = index(&entry.name);
                check_info(&entry.value, rhs[i + 2]);
                assert!(!seen[i]);
                seen[i] = true;
            }
            assert!(seen.iter().all(|seen| *seen));
            assert!(reader.next().is_none());
            reader.close()?;
        }
        7 => {
            let mut file = MatFile::create_with_format(&context, "global.mat", format)?;
            file.put_global(c"global_value", rhs[2])?;
            file.close()?;
            let mut file = MatFile::open(&context, "global.mat", OpenMode::Read)?;
            let info = file.info(c"global_value")?;
            assert!(info.is_global());
            file.close()?;
            assert!(info.is_global()); // metadata owns memory independently of file
        }
        8 => {
            let mut file = MatFile::create(&context, "early.mat")?;
            file.put(c"saved", rhs[2])?;
            if !lhs.is_empty() {
                lhs[0] = Some(123.0_f64.to_matlab());
            }
            file.put(c"", rhs[2])?; // intentional return through ?; Drop must close
            unreachable!();
        }
        9 => {
            let path = "日本語😀/値😀.mat";
            let mut file = MatFile::create_with_format(&context, path, format)?;
            file.put(c"value", rhs[2])?;
            file.close()?;
            let mut file = MatFile::open(&context, path, OpenMode::Read)?;
            lhs[0] = Some(file.get(c"value")?);
            file.close()?;
            return Ok(());
        }
        10 => {
            for mode in [OpenMode::Read, OpenMode::Update] {
                assert!(MatFile::open(&context, "nonexistent.mat", mode).is_err());
            }
            assert!(MatFile::open(&context, "corrupt.mat", OpenMode::Read).is_err());
            assert!(MatFile::create(&context, "").is_err());
            assert!(MatFile::create(&context, "a\0b").is_err());
            MatFile::create_with_format(&context, "empty.mat", format)?.close()?;
            if format == MatVersion::V4 {
                // v4 has no file header; an empty write leaves zero bytes, which
                // libmat cannot recognize as a MAT-file on reopen.
                assert!(MatFile::open(&context, "empty.mat", OpenMode::Read).is_err());
            } else {
                let mut file = MatFile::open(&context, "empty.mat", OpenMode::Read)?;
                assert!(file.variables()?.is_empty());
                file.close()?;
                let mut reader = Variables::open(&context, "empty.mat")?;
                assert!(reader.next().is_none());
                reader.close()?;
                let mut reader = VariableInfos::open(&context, "empty.mat")?;
                assert!(reader.next().is_none());
                reader.close()?;
            }
        }
        11 => {
            // SAFETY: the caller variable is only borrowed in this invocation,
            // with no MATLAB callback or workspace mutation while borrowed.
            let value = unsafe {
                rustmex::workspace::get_variable_ref(
                    rustmex::workspace::WorkSpace::Caller,
                    c"from_caller",
                )
            }
            .expect("test provides caller variable");
            let mut file = MatFile::create(&context, "caller.mat")?;
            file.put(c"from_caller", value)?;
            file.close()?;
        }
        12 => {
            let mut file = MatFile::create(&context, "panic.mat")?;
            file.put(c"saved", rhs[2])?;
            if !lhs.is_empty() {
                lhs[0] = Some(123.0_f64.to_matlab());
            }
            panic!("intentional panic after opening a file");
        }
        13 => {
            return Err(rustmex::message::AdHoc("invalid:", "日本語 %s 100%\0tail").into());
        }
        _ => panic!("unknown test command"),
    }
    if !lhs.is_empty() {
        lhs[0] = Some(0.0_f64.to_matlab());
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

fn check_info(info: &ArrayInfo<'_>, value: &mxArray) {
    assert_eq!(info.dimensions(), value.dimensions());
    assert_eq!(info.numel(), value.numel());
    assert_eq!(info.class_id(), value.raw_class_id());
    assert_eq!(info.is_complex(), value.is_complex());
    assert_eq!(info.is_sparse(), value.is_sparse());
    assert!(!info.class_name().to_bytes().is_empty());
    assert!(info.cell(usize::MAX).is_none());
    assert!(info.field(usize::MAX, c"field").is_none());
    if info.class_name() == c"struct" {
        assert_eq!(
            info.field_names(),
            vec![c"field".to_owned(), c"nested".to_owned()]
        );
        assert_eq!(info.field(0, c"field").unwrap().dimensions(), &[2, 3]);
        let nested = info.field(0, c"nested").unwrap();
        assert_eq!(nested.cell(0).unwrap().class_name(), c"double");
        assert!(info.field(0, c"unknown").is_none());
    }
    if info.class_name() == c"cell" && info.numel() != 0 {
        assert_eq!(info.cell(0).unwrap().class_name(), c"double");
        assert_eq!(info.cell(1).unwrap().class_name(), c"char");
    }
}
