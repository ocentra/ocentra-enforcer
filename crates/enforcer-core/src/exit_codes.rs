//! Mapping from foundation mechanism failures to canonical process outcomes.

use crate::error::Error;
use enforcer_domain::core_types::ExitCode;

/// Map a foundation mechanism failure to the canonical process outcome.
pub fn for_error(error: &Error) -> ExitCode {
    match error {
        Error::Decode(_) | Error::InvalidConfig(_) | Error::Env { .. } => ExitCode::ConfigError,
        Error::Io(_) | Error::Json(_) | Error::Time(_) | Error::TracingInit(_) => {
            ExitCode::InternalError
        }
    }
}

#[cfg(test)]
mod tests {
    use super::for_error;
    use crate::error::Error;
    use enforcer_domain::boundary::decode_error::DecodeError;
    use enforcer_domain::core_types::ExitCode;
    use enforcer_domain::telemetry_types::ProcessExitCode;

    #[test]
    fn exit_codes_round_trip_both_ways() {
        for exit in [
            ExitCode::Success,
            ExitCode::Violations,
            ExitCode::UsageError,
            ExitCode::ConfigError,
            ExitCode::InternalError,
        ] {
            assert_eq!(ExitCode::from_process_code(exit.process_code()), Some(exit));
        }
    }

    #[test]
    fn unknown_code_maps_to_none() {
        assert_eq!(ExitCode::from_process_code(ProcessExitCode::new(42)), None);
        assert_eq!(ExitCode::from_process_code(ProcessExitCode::new(-1)), None);
    }

    #[test]
    fn errors_map_to_documented_exit_codes() {
        let decode: Error = DecodeError::new("f", "bad").into();
        assert_eq!(for_error(&decode), ExitCode::ConfigError);
        let env = Error::Env {
            name: "X".to_owned(),
            reason: "missing".to_owned(),
        };
        assert_eq!(for_error(&env), ExitCode::ConfigError);
        let io: Error = std::io::Error::other("boom").into();
        assert_eq!(for_error(&io), ExitCode::InternalError);
        assert_eq!(
            for_error(&Error::TracingInit("dup".to_owned())),
            ExitCode::InternalError
        );
    }
}
