//! Windows-first path/time/env helpers (OcentraParent `logging-core` borrow —
//! see the vendoring attribution note in `lib.rs`).
//!
//! Downstream crates never hand-roll path handling: backslash paths are
//! normalized here once, argv-safe quoting lives here once, and clock/env
//! access routes through the shared error type.

use crate::error::{Error, Result};

/// Normalize path separators to forward slashes (`\` -> `/`).
///
/// This is the canonical internal representation: Windows accepts forward
/// slashes in APIs, and normalized paths compare/diff cleanly across
/// platforms.
pub fn normalize_separators(path: &str) -> String {
    path.replace('\\', "/")
}

/// Convert a normalized path to the native separator for display or for
/// handing to native tooling (`/` -> `\` on Windows; unchanged elsewhere).
pub fn to_native_separators(path: &str) -> String {
    if cfg!(windows) {
        path.replace('/', "\\")
    } else {
        path.to_owned()
    }
}

/// Quote a path for safe use as a single argv element on Windows shells:
/// wraps in double quotes when it contains whitespace or quotes, escaping
/// embedded double quotes.
pub fn argv_safe(path: &str) -> String {
    let needs_quoting = path.chars().any(|c| c.is_whitespace() || c == '"');
    if !needs_quoting {
        return path.to_owned();
    }
    let escaped = path.replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// Milliseconds since the Unix epoch, via the shared error type.
pub fn epoch_millis() -> Result<u128> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .map_err(|e| Error::Time(format!("system clock before Unix epoch: {e}")))
}

/// Format epoch milliseconds as an ISO-8601 UTC timestamp
/// (`YYYY-MM-DDTHH:MM:SS.mmmZ`). Pure computation, no clock access.
pub fn iso8601_utc(epoch_ms: u128) -> String {
    let total_secs = (epoch_ms / 1000) as i64;
    let millis = (epoch_ms % 1000) as u32;
    let days = total_secs.div_euclid(86_400);
    let secs_of_day = total_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

/// Read an environment variable through the shared error type.
pub fn env_var(name: &str) -> Result<String> {
    std::env::var(name).map_err(|e| Error::Env {
        name: name.to_owned(),
        reason: e.to_string(),
    })
}

/// Days-since-epoch to civil (year, month, day). Howard Hinnant's
/// `civil_from_days` algorithm; exact over the full supported range.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year_of_era = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 {
        year_of_era + 1
    } else {
        year_of_era
    };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::{
        argv_safe, env_var, epoch_millis, iso8601_utc, normalize_separators, to_native_separators,
    };
    use crate::error::{Error, Result};

    #[test]
    fn windows_backslashes_normalize_to_forward_slashes() {
        assert_eq!(
            normalize_separators(r"C:\Projects\enforcer\crates\core"),
            "C:/Projects/enforcer/crates/core"
        );
        assert_eq!(normalize_separators("already/normal"), "already/normal");
        assert_eq!(
            normalize_separators(r"mixed\and/slashes"),
            "mixed/and/slashes"
        );
    }

    #[test]
    fn native_separators_round_trip_on_windows() {
        let native = to_native_separators("C:/a/b");
        if cfg!(windows) {
            assert_eq!(native, r"C:\a\b");
        } else {
            assert_eq!(native, "C:/a/b");
        }
        assert_eq!(normalize_separators(&native), "C:/a/b");
    }

    #[test]
    fn argv_safe_quotes_only_when_needed() {
        assert_eq!(argv_safe("C:/plain/path"), "C:/plain/path");
        assert_eq!(
            argv_safe("C:/Program Files/tool"),
            "\"C:/Program Files/tool\""
        );
        assert_eq!(argv_safe("has\"quote"), "\"has\\\"quote\"");
    }

    #[test]
    fn iso8601_known_values() {
        assert_eq!(iso8601_utc(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso8601_utc(1_700_000_000_000), "2023-11-14T22:13:20.000Z");
        assert_eq!(iso8601_utc(86_400_000 + 1), "1970-01-02T00:00:00.001Z");
    }

    #[test]
    fn epoch_millis_is_sane() -> Result<()> {
        let now = epoch_millis()?;
        // After 2020-01-01 and before 2100-01-01.
        assert!(now > 1_577_836_800_000);
        assert!(now < 4_102_444_800_000);
        Ok(())
    }

    #[test]
    fn env_var_routes_through_shared_error() {
        let missing = env_var("ENFORCER_CORE_TEST_DEFINITELY_UNSET_VAR");
        assert!(matches!(missing, Err(Error::Env { .. })));
    }
}
