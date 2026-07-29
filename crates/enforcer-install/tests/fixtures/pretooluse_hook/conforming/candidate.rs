// Conforming fixture (workpack c04): typed `Result` propagation, no
// unwrap/expect/panic!/todo!/unimplemented!/dbg! in first-party code -- the
// hook must ALLOW this edit.
fn parse_it(raw: &str) -> Result<i32, std::num::ParseIntError> {
    let value = raw.parse::<i32>()?;
    Ok(value)
}
