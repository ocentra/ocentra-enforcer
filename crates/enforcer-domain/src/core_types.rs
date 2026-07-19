//! Canonical semantic values shared by the workspace foundation mechanisms.
//!
//! `enforcer-core` owns behavior (budget arithmetic, hash computation, I/O,
//! tracing, and run-context gates). This module owns the values that cross
//! those mechanism boundaries so consumers import one canonical type directly.

use crate::boundary::decode_error::DecodeError;
use crate::telemetry_types::ProcessExitCode;

macro_rules! count_value {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
            serde::Serialize, serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub(crate) usize);

        impl From<$name> for usize {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

count_value!(
    /// Number of registered tool descriptors in one measured surface.
    ToolDescriptorCount
);
count_value!(
    /// Exact serialized byte length of one measured tool surface.
    ToolSurfaceByteCount
);
count_value!(
    /// Estimated token count derived for one measured tool surface.
    EstimatedTokenCount
);

/// Version of the committed context-budget baseline wire format.
#[doc = "SERIALIZATION-DOC: encoded as a positive JSON integer schema version."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct BudgetBaselineVersion(std::num::NonZeroU32);

impl BudgetBaselineVersion {
    /// Initial context-budget baseline schema.
    pub const V1: Self = Self(std::num::NonZeroU32::MIN);
}

impl<'de> serde::Deserialize<'de> for BudgetBaselineVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = <u32 as serde::Deserialize>::deserialize(deserializer)?;
        std::num::NonZeroU32::new(raw)
            .map(Self)
            .ok_or_else(|| serde::de::Error::custom("budget baseline version must be positive"))
    }
}

/// Canonical version of the currently supported budget-baseline wire shape.
pub const BUDGET_BASELINE_VERSION: BudgetBaselineVersion = BudgetBaselineVersion::V1;

/// Non-negative finite growth tolerance expressed as a percentage.
#[doc = "SERIALIZATION-DOC: encoded as a finite non-negative JSON number."]
#[doc = "BRAND-INVARIANT: finite percentage greater than or equal to zero."]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, serde::Serialize)]
#[serde(transparent)]
pub struct GrowthTolerancePct(pub(crate) f64);

impl From<GrowthTolerancePct> for f64 {
    fn from(value: GrowthTolerancePct) -> Self {
        value.0
    }
}

impl<'de> serde::Deserialize<'de> for GrowthTolerancePct {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = <f64 as serde::Deserialize>::deserialize(deserializer)?;
        crate::boundary::core::growth_tolerance(raw).map_err(serde::de::Error::custom)
    }
}

/// Signed byte change between a measurement and its baseline.
#[doc = "BRAND-INVARIANT: signed exact byte delta; zero and negative values are valid."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolSurfaceByteDelta(pub(crate) i64);

impl From<ToolSurfaceByteDelta> for i64 {
    fn from(value: ToolSurfaceByteDelta) -> Self {
        value.0
    }
}

/// Calculated growth percentage. Positive infinity represents growth over a
/// zero-byte baseline and therefore always fails the ratchet.
#[doc = "BRAND-INVARIANT: context-budget arithmetic result; NaN is normalized fail-closed."]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ToolSurfaceGrowthPct(pub(crate) f64);

impl From<ToolSurfaceGrowthPct> for f64 {
    fn from(value: ToolSurfaceGrowthPct) -> Self {
        value.0
    }
}

/// Unit-interval score or confidence emitted by an advisory mechanism.
#[doc = "BRAND-INVARIANT: finite value clamped to the inclusive range zero through one."]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct UnitInterval(pub(crate) f64);

impl UnitInterval {
    pub const ZERO: Self = Self(0.0);
    pub const ONE: Self = Self(1.0);
}

impl From<UnitInterval> for f64 {
    fn from(value: UnitInterval) -> Self {
        value.0
    }
}

/// One measured tool-surface snapshot.
#[doc = "SERIALIZATION-DOC: camelCase object containing exact tool, byte, and token counts."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeasuredSurface {
    tool_count: ToolDescriptorCount,
    total_bytes: ToolSurfaceByteCount,
    estimated_tokens: EstimatedTokenCount,
}

impl MeasuredSurface {
    /// Build a measurement from canonical count values.
    pub const fn from_canonical_counts(
        tool_count: ToolDescriptorCount,
        total_bytes: ToolSurfaceByteCount,
        estimated_tokens: EstimatedTokenCount,
    ) -> Self {
        Self {
            tool_count,
            total_bytes,
            estimated_tokens,
        }
    }

    /// Number of descriptors represented by this measurement.
    pub const fn tool_count(self) -> ToolDescriptorCount {
        self.tool_count
    }

    /// Exact serialized byte length represented by this measurement.
    pub const fn total_bytes(self) -> ToolSurfaceByteCount {
        self.total_bytes
    }

    /// Estimated token count represented by this measurement.
    pub const fn estimated_tokens(self) -> EstimatedTokenCount {
        self.estimated_tokens
    }
}

impl<'de> serde::Deserialize<'de> for MeasuredSurface {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = crate::boundary::core::MeasuredSurfaceWire::deserialize(deserializer)?;
        if wire.estimated_tokens != wire.total_bytes / 4 {
            return Err(serde::de::Error::custom(
                "estimatedTokens must equal totalBytes divided by four",
            ));
        }
        Ok(crate::boundary::core::measured_surface(
            wire.tool_count,
            wire.total_bytes,
        ))
    }
}

/// Committed, reviewed baseline for the context-budget ratchet.
#[doc = "SERIALIZATION-DOC: camelCase baseline object with validated version, surface, and tolerance."]
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetBaseline {
    version: BudgetBaselineVersion,
    surface: MeasuredSurface,
    tolerance_pct: GrowthTolerancePct,
}

impl<'de> serde::Deserialize<'de> for BudgetBaseline {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = crate::boundary::core::BudgetBaselineWire::deserialize(deserializer)?;
        Ok(Self::new(wire.version, wire.surface, wire.tolerance_pct))
    }
}

/// Closed verdict produced by the context-budget ratchet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetGateDecision {
    Pass,
    Fail,
}

impl BudgetBaseline {
    /// Construct a validated baseline from its canonical values.
    pub const fn new(
        version: BudgetBaselineVersion,
        surface: MeasuredSurface,
        tolerance_pct: GrowthTolerancePct,
    ) -> Self {
        Self {
            version,
            surface,
            tolerance_pct,
        }
    }

    /// Baseline wire version.
    pub const fn version(self) -> BudgetBaselineVersion {
        self.version
    }

    /// Reference measurement.
    pub const fn surface(self) -> MeasuredSurface {
        self.surface
    }

    /// Allowed growth percentage.
    pub const fn tolerance_pct(self) -> GrowthTolerancePct {
        self.tolerance_pct
    }
}

/// Outcome values produced by the context-budget evaluator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetGateOutcome {
    measured: MeasuredSurface,
    baseline: BudgetBaseline,
    byte_delta: ToolSurfaceByteDelta,
    growth_pct: ToolSurfaceGrowthPct,
}

impl BudgetGateOutcome {
    /// Assemble an outcome from trusted evaluator arithmetic.
    pub const fn new(
        measured: MeasuredSurface,
        baseline: BudgetBaseline,
        byte_delta: ToolSurfaceByteDelta,
        growth_pct: ToolSurfaceGrowthPct,
    ) -> Self {
        Self {
            measured,
            baseline,
            byte_delta,
            growth_pct,
        }
    }

    pub const fn measured(self) -> MeasuredSurface {
        self.measured
    }

    pub const fn baseline(self) -> BudgetBaseline {
        self.baseline
    }

    pub const fn byte_delta(self) -> ToolSurfaceByteDelta {
        self.byte_delta
    }

    pub const fn growth_pct(self) -> ToolSurfaceGrowthPct {
        self.growth_pct
    }
}

/// Advisory context-budget efficiency signal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EfficiencyScore {
    score: UnitInterval,
    confidence: UnitInterval,
}

impl EfficiencyScore {
    /// Assemble an advisory score from canonical unit-interval values.
    pub const fn from_intervals(score: UnitInterval, confidence: UnitInterval) -> Self {
        Self { score, confidence }
    }

    pub const fn score(self) -> UnitInterval {
        self.score
    }

    pub const fn confidence(self) -> UnitInterval {
        self.confidence
    }
}

/// Stable process outcome taxonomy shared by executable surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success,
    Violations,
    UsageError,
    ConfigError,
    InternalError,
}

impl ExitCode {
    /// Map the semantic outcome to its operating-system process code.
    pub const fn process_code(self) -> ProcessExitCode {
        ProcessExitCode::new(match self {
            Self::Success => 0,
            Self::Violations => 1,
            Self::UsageError => 2,
            Self::ConfigError => 78,
            Self::InternalError => 70,
        })
    }

    /// Recover a known semantic outcome from a process-code boundary value.
    pub const fn from_process_code(code: ProcessExitCode) -> Option<Self> {
        match code.get() {
            0 => Some(Self::Success),
            1 => Some(Self::Violations),
            2 => Some(Self::UsageError),
            78 => Some(Self::ConfigError),
            70 => Some(Self::InternalError),
            _ => None,
        }
    }
}

/// Closed silent-vs-human execution signal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RunContext {
    #[default]
    AgentInline,
    HumanReview,
}

pub const RUN_CONTEXT_ENV_VAR: &str = "ENFORCER_RUN_CONTEXT";
pub const AGENT_INLINE_TOKEN: &str = "agent-inline";
pub const HUMAN_REVIEW_TOKEN: &str = "human-review";

impl RunContext {
    /// Canonical boundary token for this execution context.
    pub const fn as_token(self) -> &'static str {
        match self {
            Self::AgentInline => AGENT_INLINE_TOKEN,
            Self::HumanReview => HUMAN_REVIEW_TOKEN,
        }
    }
}

impl std::str::FromStr for RunContext {
    type Err = DecodeError;

    fn from_str(raw: &str) -> Result<Self, DecodeError> {
        match raw {
            AGENT_INLINE_TOKEN => Ok(Self::AgentInline),
            HUMAN_REVIEW_TOKEN => Ok(Self::HumanReview),
            other => Err(DecodeError::new(
                "runContext",
                format!(
                    "unrecognized run-context value `{other}`; expected \
                     `{AGENT_INLINE_TOKEN}` or `{HUMAN_REVIEW_TOKEN}`"
                ),
            )
            .with_input_hint(other)),
        }
    }
}

impl serde::Serialize for RunContext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_token())
    }
}

impl<'de> serde::Deserialize<'de> for RunContext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = <String as serde::Deserialize>::deserialize(deserializer)?;
        <Self as std::str::FromStr>::from_str(&raw).map_err(serde::de::Error::custom)
    }
}

/// Zero-based position of a link in a hash chain.
#[doc = "BRAND-INVARIANT: exact zero-based chain position; zero is valid."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChainLinkIndex(pub(crate) usize);

impl From<ChainLinkIndex> for usize {
    fn from(value: ChainLinkIndex) -> Self {
        value.0
    }
}

impl From<ChainLinkIndex> for crate::memory_types::MemoryErrorLineIndex {
    fn from(value: ChainLinkIndex) -> Self {
        // BRAND-INVARIANT: ChainLinkIndex is a validated zero-based chain position;
        // converting through usize preserves that domain meaning for the memory line index.
        usize::from(value).into()
    }
}

/// Number of entries observed in one side of a persisted hash chain.
#[doc = "BRAND-INVARIANT: exact non-negative entry count; zero is valid."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChainEntryCount(pub(crate) usize);

impl From<ChainEntryCount> for usize {
    fn from(value: ChainEntryCount) -> Self {
        value.0
    }
}

impl std::fmt::Display for ChainEntryCount {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// First mismatch found while verifying a hash chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainBreak {
    DigestMismatch {
        index: ChainLinkIndex,
        recorded: crate::hashes::Sha256,
        expected: crate::hashes::Sha256,
    },
    LengthMismatch {
        index: ChainLinkIndex,
        recorded_digests: ChainEntryCount,
        data_lines: ChainEntryCount,
    },
}

impl ChainBreak {
    pub const fn digest_mismatch(
        index: ChainLinkIndex,
        recorded: crate::hashes::Sha256,
        expected: crate::hashes::Sha256,
    ) -> Self {
        Self::DigestMismatch {
            index,
            recorded,
            expected,
        }
    }

    pub const fn length_mismatch(
        index: ChainLinkIndex,
        recorded_digests: ChainEntryCount,
        data_lines: ChainEntryCount,
    ) -> Self {
        Self::LengthMismatch {
            index,
            recorded_digests,
            data_lines,
        }
    }

    pub const fn index(&self) -> ChainLinkIndex {
        match self {
            Self::DigestMismatch { index, .. } | Self::LengthMismatch { index, .. } => *index,
        }
    }
}
