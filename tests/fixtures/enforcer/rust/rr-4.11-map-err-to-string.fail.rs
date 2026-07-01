// RR-4.11
pub fn load_user() -> Result<(), String> {
    std::fs::read_to_string("x").map_err(|e| e.to_string())?;
    Ok(())
}
