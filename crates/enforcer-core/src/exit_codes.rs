//! Process exit codes shared by every enforcer binary surface (CLI, MCP).
//!
//! The mapping is part of the consumer contract: scripts and CI gates key on
//! these numbers, so they are typed here once and mapped from [`crate::error::Error`]
//! in exactly one place.

use crate::error::Error;

/// Typed process exit codes for enforcer binaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// Clean run, no findings.
    Success,
    /// Run completed and produced blocking findings/violations.
    Violations,
    /// Caller misuse: bad flags, unknown subcommand, malformed request.
    UsageError,
    /// Configuration could not be loaded/validated.
    ConfigError,
    /// Internal failure (I/O, serialization, bugs).
    InternalError,
}

impl ExitCode {
    /// Numeric process exit code.
    pub fn code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::Violations => 1,
            Self::UsageError => 2,
            Self::ConfigError => 78,
            Self::InternalError => 70,
        }
    }

    /// Reverse mapping from a raw process code, when it is one of ours.
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::Success),
            1 => Some(Self::Violations),
            2 => Some(Self::UsageError),
            78 => Some(Self::ConfigError),
            70 => Some(Self::InternalError),
            _ => None,
        }
    }
}

impl From<&Error> for ExitCode {
    fn from(err: &Error) -> Self {
        match err {
            Error::Decode(_) | Error::InvalidConfig(_) | Error::Env { .. } => Self::ConfigError,
            Error::Io(_) | Error::Json(_) | Error::Time(_) | Error::TracingInit(_) => {
                Self::InternalError
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ExitCode;
    use crate::error::{DecodeError, Error};

    #[test]
    fn exit_codes_round_trip_both_ways() {
        let all = [
            ExitCode::Success,
            ExitCode::Violations,
            ExitCode::UsageError,
            ExitCode::ConfigError,
            ExitCode::InternalError,
        ];
        for exit in all {
            assert_eq!(ExitCode::from_code(exit.code()), Some(exit));
        }
    }

    #[test]
    fn unknown_code_maps_to_none() {
        assert_eq!(ExitCode::from_code(42), None);
        assert_eq!(ExitCode::from_code(-1), None);
    }

    #[test]
    fn errors_map_to_documented_exit_codes() {
        let decode: Error = DecodeError::new("f", "bad").into();
        assert_eq!(ExitCode::from(&decode), ExitCode::ConfigError);

        let env = Error::Env {
            name: "X".to_owned(),
            reason: "missing".to_owned(),
        };
        assert_eq!(ExitCode::from(&env), ExitCode::ConfigError);

        let io: Error = std::io::Error::other("boom").into();
        assert_eq!(ExitCode::from(&io), ExitCode::InternalError);

        let tracing_init = Error::TracingInit("dup".to_owned());
        assert_eq!(ExitCode::from(&tracing_init), ExitCode::InternalError);
    }
}
