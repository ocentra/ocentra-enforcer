// FAIL fixture for RUST-ERR-NONEXHAUSTIVE: public error enum missing
// #[non_exhaustive].
pub enum ConfigError {
    NotFound,
    Invalid(String),
}
