// FAIL fixture for RUST-ERR-MSG-STYLE: error message capitalized and with
// trailing punctuation.
#[derive(Debug, thiserror::Error)]
enum ConfigError {
    #[error("File Not Found.")]
    NotFound,
}
