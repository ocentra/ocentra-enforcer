//! Severity and enforcement-tier enums.
//!
//! These remain closed canonical values. Durable/API boundaries deliberately
//! render them as strings through the explicit mappings below; arbitrary wire
//! strings never become a domain variant by derive magic.

/// Finding severity, lowercase on the wire (`"error"`, `"warning"`,
/// `"info"`) to match the legacy `.mjs` report shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, ts_rs::TS)]
#[ts(type = "string")]
#[doc = "The closed severity classification emitted with a finding."]
pub enum Severity {
    /// Blocking violation.
    Error,
    /// Non-blocking warning.
    Warning,
    /// Informational note.
    Info,
}

/// Mechanical-enforcement tier (doctrine: T1 typed/compile-time, T2 scored
/// scan, T3 review-assist). Wire form is `"T1"`/`"T2"`/`"T3"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, ts_rs::TS)]
#[ts(type = "string")]
#[doc = "The closed tier describing how a rule is mechanically enforced."]
pub enum Tier {
    /// Typed / compile-time / hard-gate enforcement.
    T1,
    /// Scored scan enforcement.
    T2,
    /// Review-assist enforcement.
    T3,
}

impl Severity {
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

impl Tier {
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::T1 => "T1",
            Self::T2 => "T2",
            Self::T3 => "T3",
        }
    }
}

macro_rules! closed_string_wire {
    ($value:ty, $name:literal, { $($wire:literal => $variant:path),+ $(,)? }) => {
        impl serde::Serialize for $value {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(self.wire_name())
            }
        }

        impl<'de> serde::Deserialize<'de> for $value {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let raw = String::deserialize(deserializer)?;
                match raw.as_str() {
                    $($wire => Ok($variant)),+,
                    _ => Err(serde::de::Error::custom(format!("invalid {} `{raw}`", $name))),
                }
            }
        }
    };
}

closed_string_wire!(Severity, "severity", {
    "error" => Severity::Error,
    "warning" => Severity::Warning,
    "info" => Severity::Info,
});
closed_string_wire!(Tier, "tier", {
    "T1" => Tier::T1,
    "T2" => Tier::T2,
    "T3" => Tier::T3,
});
