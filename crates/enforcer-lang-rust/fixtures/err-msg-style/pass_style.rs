// PASS fixture for RUST-ERR-MSG-STYLE: lowercase, no trailing punctuation.
#[derive(Debug, thiserror::Error)]
enum ConfigError {
    #[error("config file not found")]
    NotFound,
}
