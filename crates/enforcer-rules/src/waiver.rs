//! Typed, fail-closed waivers for individual findings.
//!
//! This is intentionally not a second policy-toggle format. A waiver names
//! one known [`RuleId`], one exact repository-relative path, an accountable
//! owner, and a non-empty reason. Expired waivers never match a finding.

use std::collections::BTreeSet;
use std::path::Path;
use std::str::FromStr;

use enforcer_domain::ids::RuleId;

use crate::registry::RuleRegistry;

/// A calendar date used for waiver expiry. Dates are UTC calendar dates and
/// use the inclusive rule: a waiver remains active through its expiry date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WaiverDate {
    year: u16,
    month: u8,
    day: u8,
}

impl WaiverDate {
    /// Build a checked UTC calendar date.
    pub fn new(year: u16, month: u8, day: u8) -> Result<Self, WaiverLoadError> {
        let valid_month = (1..=12).contains(&month);
        let valid_day = valid_month && (1..=days_in_month(year, month)).contains(&day);
        if !valid_day {
            return Err(WaiverLoadError::InvalidExpiry {
                value: format!("{year:04}-{month:02}-{day:02}"),
            });
        }
        Ok(Self { year, month, day })
    }
}

impl std::fmt::Display for WaiverDate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

impl FromStr for WaiverDate {
    type Err = WaiverLoadError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        if raw.len() != 10
            || raw.as_bytes().get(4) != Some(&b'-')
            || raw.as_bytes().get(7) != Some(&b'-')
        {
            return Err(WaiverLoadError::InvalidExpiry {
                value: raw.to_owned(),
            });
        }
        let year =
            raw[..4]
                .parse::<u16>()
                .map_err(|_parse_error| WaiverLoadError::InvalidExpiry {
                    value: raw.to_owned(),
                })?;
        let month =
            raw[5..7]
                .parse::<u8>()
                .map_err(|_parse_error| WaiverLoadError::InvalidExpiry {
                    value: raw.to_owned(),
                })?;
        let day =
            raw[8..10]
                .parse::<u8>()
                .map_err(|_parse_error| WaiverLoadError::InvalidExpiry {
                    value: raw.to_owned(),
                })?;
        Self::new(year, month, day)
    }
}

impl serde::Serialize for WaiverDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for WaiverDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::from_str(&raw).map_err(serde::de::Error::custom)
    }
}

/// Determines whether an expired registry entry rejects the full load or is
/// retained for audit while remaining ineligible to match a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiryPolicy {
    /// Reject a registry containing an expired entry.
    RejectExpired,
    /// Retain expired entries for audit. They still never match a finding.
    RetainExpiredForAudit,
}

/// One auditable exception for one rule and one exact repository-relative
/// path. The shape cannot represent a numeric policy limit bump.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Waiver {
    /// Exact repository-relative file path the waiver applies to.
    pub path: String,
    /// Branded identifier for the specific waived rule.
    pub rule_id: RuleId,
    /// Accountable human or team.
    pub owner: String,
    /// Concrete reason this one finding is temporarily accepted.
    pub reason: String,
    /// Optional inclusive UTC expiry date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<WaiverDate>,
}

/// The on-disk waiver document. JSON is used because the crate already owns
/// a serde JSON loader convention; no RON dependency or workspace lockfile
/// mutation is needed for this standalone registry.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WaiverRegistry {
    /// Every tracked waiver entry.
    #[serde(default)]
    pub waivers: Vec<Waiver>,
}

impl WaiverRegistry {
    /// Parse, validate, and return a registry. A partially valid document
    /// never escapes this boundary.
    pub fn parse(
        raw: &str,
        source: &str,
        rules: &RuleRegistry,
        today: WaiverDate,
        expiry_policy: ExpiryPolicy,
    ) -> WaiverResult<Self> {
        let registry =
            serde_json::from_str::<Self>(raw).map_err(|error| WaiverLoadError::Parse {
                path: source.to_owned(),
                reason: error.to_string(),
            })?;
        registry.validate(rules, today, expiry_policy)?;
        Ok(registry)
    }

    /// Read, parse, and validate a registry from disk.
    pub fn load_file(
        path: &Path,
        rules: &RuleRegistry,
        today: WaiverDate,
        expiry_policy: ExpiryPolicy,
    ) -> WaiverResult<Self> {
        let display = path.display().to_string();
        let raw = std::fs::read_to_string(path).map_err(|error| WaiverLoadError::Io {
            path: display.clone(),
            reason: error.to_string(),
        })?;
        Self::parse(&raw, &display, rules, today, expiry_policy)
    }

    /// Validate all semantic constraints that serde alone cannot express.
    pub fn validate(
        &self,
        rules: &RuleRegistry,
        today: WaiverDate,
        expiry_policy: ExpiryPolicy,
    ) -> WaiverResult<()> {
        let mut entries = BTreeSet::new();
        for waiver in &self.waivers {
            let path = canonical_relative_path(&waiver.path)?;
            if waiver.owner.trim().is_empty() {
                return Err(WaiverLoadError::EmptyOwner {
                    path: waiver.path.clone(),
                    rule_id: waiver.rule_id.to_string(),
                });
            }
            if waiver.reason.trim().is_empty() {
                return Err(WaiverLoadError::EmptyReason {
                    path: waiver.path.clone(),
                    rule_id: waiver.rule_id.to_string(),
                });
            }
            if rules.get(&waiver.rule_id).is_none() {
                return Err(WaiverLoadError::UnknownRuleId {
                    path: waiver.path.clone(),
                    rule_id: waiver.rule_id.to_string(),
                });
            }
            if matches!(expiry_policy, ExpiryPolicy::RejectExpired)
                && waiver.expires.is_some_and(|expires| expires < today)
            {
                return Err(WaiverLoadError::Expired {
                    path: waiver.path.clone(),
                    rule_id: waiver.rule_id.to_string(),
                    expires: waiver
                        .expires
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                });
            }
            if !entries.insert((path, waiver.rule_id.clone())) {
                return Err(WaiverLoadError::DuplicateScope {
                    path: waiver.path.clone(),
                    rule_id: waiver.rule_id.to_string(),
                });
            }
        }
        Ok(())
    }

    /// Return the single active waiver applicable to this exact path and
    /// rule. Invalid candidate paths and expired entries fail closed.
    pub fn matching<'a>(
        &'a self,
        path: &str,
        rule_id: &RuleId,
        today: WaiverDate,
    ) -> Option<&'a Waiver> {
        let path = canonical_relative_path(path).ok()?;
        self.waivers.iter().find(|waiver| {
            waiver.rule_id == *rule_id
                && canonical_relative_path(&waiver.path).ok().as_deref() == Some(path.as_str())
                && waiver.expires.is_none_or(|expires| expires >= today)
        })
    }
}

/// Fail-closed result for waiver registry loading and validation.
pub type WaiverResult<T> = Result<T, WaiverLoadError>;

/// A registry boundary failure.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum WaiverLoadError {
    /// The document could not be read.
    #[error("failed to read waiver registry `{path}`: {reason}")]
    Io { path: String, reason: String },
    /// The document did not decode into the strict registry shape.
    #[error("waiver registry parse failed at `{path}`: {reason}")]
    Parse { path: String, reason: String },
    /// An expiry date did not use a real `YYYY-MM-DD` calendar date.
    #[error("invalid waiver expiry `{value}`; expected a real YYYY-MM-DD date")]
    InvalidExpiry { value: String },
    /// A waiver path was broad, absolute, or escaped the project root.
    #[error("waiver path `{path}` must be a narrow repository-relative file path")]
    InvalidPath { path: String },
    /// A waiver omitted accountable ownership.
    #[error("waiver for `{path}` / `{rule_id}` has an empty owner")]
    EmptyOwner { path: String, rule_id: String },
    /// A waiver omitted an auditable reason.
    #[error("waiver for `{path}` / `{rule_id}` has an empty reason")]
    EmptyReason { path: String, rule_id: String },
    /// A syntactically valid identifier was not present in the rule registry.
    #[error("waiver for `{path}` references unknown rule `{rule_id}`")]
    UnknownRuleId { path: String, rule_id: String },
    /// A strict load rejected a waiver after its expiry date.
    #[error("waiver for `{path}` / `{rule_id}` expired on {expires}")]
    Expired {
        path: String,
        rule_id: String,
        expires: String,
    },
    /// Two entries tried to waive the same rule for the same path.
    #[error("duplicate waiver scope for `{path}` / `{rule_id}`")]
    DuplicateScope { path: String, rule_id: String },
}

fn canonical_relative_path(raw: &str) -> WaiverResult<String> {
    let normalized = raw.trim().replace('\\', "/");
    let normalized = normalized.trim_start_matches("./");
    let invalid = normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains(':')
        || normalized.contains('*')
        || normalized
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..");
    if invalid {
        return Err(WaiverLoadError::InvalidPath {
            path: raw.to_owned(),
        });
    }
    Ok(normalized.to_owned())
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}
