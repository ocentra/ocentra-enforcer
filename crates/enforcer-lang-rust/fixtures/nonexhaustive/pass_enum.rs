// PASS fixture for RUST-ERR-NONEXHAUSTIVE: public error enum carries
// #[non_exhaustive].
#[non_exhaustive]
pub enum ConfigError {
    NotFound,
    Invalid(String),
}
