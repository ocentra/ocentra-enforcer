// PASS fixture for RUST-LAYER-1.1: pure domain file, no forbidden-crate
// imports, no I/O macros.
pub struct Config {
    pub name: String,
}

pub fn describe(config: &Config) -> String {
    format!("config: {}", config.name)
}
