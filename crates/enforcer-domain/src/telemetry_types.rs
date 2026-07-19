//! Canonical scalar values shared by durable telemetry and audit records.

use std::num::NonZeroU32;

use crate::boundary::decode_error::DecodeError;

macro_rules! scalar_value {
    ($(#[$doc:meta])* $name:ident, $inner:ty) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
            serde::Serialize, serde::Deserialize, ts_rs::TS,
        )]
        #[serde(transparent)]
        #[ts(type = "number")]
        pub struct $name($inner);

        impl $name {
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            pub const fn get(self) -> $inner {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

scalar_value!(
    #[doc = "Milliseconds since the Unix epoch."]
    EpochMillis,
    u64
);

scalar_value!(
    #[doc = "Elapsed wall-clock duration measured in milliseconds."]
    DurationMillis,
    u64
);
scalar_value!(
    #[doc = "Operating-system process exit code."]
    ProcessExitCode,
    i32
);
scalar_value!(
    #[doc = "Count of findings in a telemetry projection."]
    FindingCount,
    u64
);
scalar_value!(
    #[doc = "Count of files in a telemetry projection."]
    FileCount,
    u64
);
scalar_value!(
    #[doc = "Count of rules selected for one run."]
    RuleCount,
    u32
);

/// Positive schema version carried by durable record envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, ts_rs::TS)]
#[ts(type = "number")]
#[doc = "Canonical domain representation for RecordSchemaVersion."]
pub struct RecordSchemaVersion(NonZeroU32);

impl RecordSchemaVersion {
    pub const V1: Self = Self(NonZeroU32::MIN);

    /// Brand an already validated positive record schema version.
    pub const fn try_new(value: NonZeroU32) -> Self {
        Self(value)
    }
}

impl serde::Serialize for RecordSchemaVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(self.0.get())
    }
}

impl<'de> serde::Deserialize<'de> for RecordSchemaVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <u32 as serde::Deserialize>::deserialize(deserializer)?;
        NonZeroU32::new(value)
            .map(Self)
            .ok_or_else(|| serde::de::Error::custom("schemaVersion must be greater than zero"))
    }
}

/// Positive 1-based source line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, ts_rs::TS)]
#[ts(type = "number")]
#[doc = "Canonical domain representation for SourceLine."]
pub struct SourceLine(NonZeroU32);

impl SourceLine {
    /// Brand an already validated positive one-based source line.
    pub const fn try_new(value: NonZeroU32) -> Self {
        Self(value)
    }

    /// Read the validated positive one-based source line.
    #[must_use]
    pub const fn value(self) -> NonZeroU32 {
        self.0
    }
}

impl std::fmt::Display for SourceLine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.get().fmt(formatter)
    }
}

impl serde::Serialize for SourceLine {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(self.0.get())
    }
}

impl<'de> serde::Deserialize<'de> for SourceLine {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <u32 as serde::Deserialize>::deserialize(deserializer)?;
        NonZeroU32::new(value)
            .map(Self)
            .ok_or_else(|| serde::de::Error::custom("sourceLine must be greater than zero"))
    }
}

/// Validated command/subcommand recorded for a run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, ts_rs::TS)]
#[ts(type = "string")]
#[doc = "Canonical domain representation for RunCommandName."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct RunCommandName(String);

impl RunCommandName {
    /// Validate a recorded command, rejecting invalid blank or control-bearing text.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        if value.trim().is_empty() || value.chars().any(char::is_control) {
            return Err(DecodeError::new(
                "runCommand",
                "must be non-empty printable text",
            ));
        }
        Ok(Self(value))
    }

    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl serde::Serialize for RunCommandName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for RunCommandName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::try_new(value).map_err(serde::de::Error::custom)
    }
}

/// Fixed record kind for the single-shape run telemetry stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ts_rs::TS)]
#[doc = "Canonical domain representation for RunRecordKind."]
pub enum RunRecordKind {
    Run,
}

impl serde::Serialize for RunRecordKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str("run")
    }
}

impl<'de> serde::Deserialize<'de> for RunRecordKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        if value == "run" {
            Ok(Self::Run)
        } else {
            Err(serde::de::Error::custom("run record kind must be `run`"))
        }
    }
}
