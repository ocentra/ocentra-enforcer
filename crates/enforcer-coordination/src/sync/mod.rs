//! Append-only stream persistence, hash verification, and retention.

pub mod retention;
#[path = "peer/boundary.rs"]
pub mod peer;
#[path = "boundary.rs"]
pub mod stream;
