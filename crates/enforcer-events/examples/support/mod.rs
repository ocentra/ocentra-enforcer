use std::io;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_events::error::EventingError;

/// Event-runtime variants for example error.
#[derive(Debug, thiserror::Error)]
pub enum ExampleError {
    #[error("example value decode failed: {0}")]
    Decode(#[from] DecodeError),
    #[error("event example failed: {0}")]
    Eventing(#[from] EventingError),
    #[error("example output failed: {0}")]
    Io(#[from] io::Error),
}
