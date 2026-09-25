use matlas::{Error, ErrorKind, Inputs, Matlab, Outputs, Result};

matlas::mex_entrypoint!(run);

fn run<'mex>(
    cx: &mut Matlab<'mex>,
    inputs: Inputs<'mex, 1>,
    outputs: &mut Outputs<'mex, 1>,
) -> Result<()> {
    let [value] = inputs.into_array();
    if value.is_text_scalar() {
        let text = value.to_text(cx)?;
        let [] = cx.call_array::<0>(c"drawnow", &[])?;
        let [_rows, _columns] = cx.call_array::<2>(c"size", &[value])?;
        outputs.set(0, cx.string(&text)?)?;
    } else if value.is_logical_scalar() {
        outputs.set(0, cx.logical_scalar(value.logical_scalar()?)?)?;
    } else if value.is_numeric() {
        return Err(
            Error::new(ErrorKind::InvalidInput, "fixed MEX", "numeric input")
                .with_id("matlasTest:CustomInput")?,
        );
    } else {
        return Err(Error::new(
            ErrorKind::Type,
            "fixed MEX",
            "unsupported input",
        ));
    }
    Ok(())
}
