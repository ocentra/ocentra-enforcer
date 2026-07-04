// PASS fixture for T1-RUSTERR.1: typed Result propagation via `?`, no
// unwrap/expect/panic!/todo!/unimplemented!/dbg! in first-party code.
fn parse_it(raw: &str) -> Result<i32, std::num::ParseIntError> {
    let value = raw.parse::<i32>()?;
    Ok(value)
}
