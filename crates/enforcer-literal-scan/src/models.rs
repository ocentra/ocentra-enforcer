use std::collections::BTreeMap;

use enforcer_domain::ids::{BuiltInLiteralRule, BuiltInSecurityRule, RuleId};
use enforcer_domain::scan_types::{
    LiteralBasenameSet, LiteralConfidence, LiteralExtensionSet,
    LiteralFileByteLimit, LiteralFileRole as FileRole, LiteralFindingDisposition,
    LiteralFindingHash, LiteralFindingPath, LiteralFindingReason, LiteralFindingSuggestion,
    LiteralLanguageFamily as LanguageFamily, LiteralLanguageId, LiteralLanguageName,
    LiteralPreview, LiteralRiskCategory as RiskCategory, LiteralRiskScore, LiteralScanCommand,
    LiteralScanCount, LiteralScanDurationMillis, LiteralScanOutputFormat as OutputFormat,
    LiteralScanPaths, LiteralScanRoot, LiteralScanToggle, LiteralSourceColumn,
    LiteralSourceContext, LiteralSourceLine, LiteralSourceText, LiteralStringSyntaxProfile,
    LiteralSyntaxKind as LiteralKind,
};
use enforcer_domain::findings::ReportOutcome;
use enforcer_domain::severity::Severity;

/// Fully decoded command-line request consumed by the literal scanner.
#[derive(Debug, Clone)]
pub struct CliOptions {
    pub command: LiteralScanCommand,
    pub root: LiteralScanRoot,
    pub files: LiteralScanPaths,
    pub output_format: OutputFormat,
    pub min_score: LiteralRiskScore,
    pub include_low: LiteralScanToggle,
    pub include_ignored: LiteralScanToggle,
    pub include_unknown_code: LiteralScanToggle,
    pub respect_gitignore: LiteralScanToggle,
    pub max_file_bytes: LiteralFileByteLimit,
    pub fail_above: Option<LiteralRiskScore>,
    pub languages: Vec<LiteralLanguageId>,
    pub explain_category: Option<RiskCategory>,
    pub help: LiteralScanToggle,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            command: LiteralScanCommand::Scan,
            root: LiteralScanRoot::default(),
            files: LiteralScanPaths::default(),
            output_format: OutputFormat::Json,
            min_score: LiteralRiskScore::default(),
            include_low: LiteralScanToggle::Disabled,
            include_ignored: LiteralScanToggle::Disabled,
            include_unknown_code: LiteralScanToggle::Disabled,
            respect_gitignore: LiteralScanToggle::Enabled,
            max_file_bytes: LiteralFileByteLimit::default(),
            fail_above: None,
            languages: Vec::new(),
            explain_category: None,
            help: LiteralScanToggle::Disabled,
        }
    }
}

/// One curated lexer registration.
#[derive(Debug, Clone, Copy)]
pub struct LanguageSpec {
    pub id: LiteralLanguageName,
    pub family: LanguageFamily,
    pub extensions: LiteralExtensionSet,
    pub basenames: LiteralBasenameSet,
    pub syntax: LiteralStringSyntaxProfile,
}

/// One literal extracted from source before semantic classification.
#[derive(Debug, Clone)]
pub struct LiteralCandidate {
    pub text: LiteralSourceText,
    pub line: LiteralSourceLine,
    pub column: LiteralSourceColumn,
    pub kind: LiteralKind,
    pub context: LiteralSourceContext,
}

pub(crate) fn rule_id_for_category(category: RiskCategory) -> RuleId {
        match category {
            RiskCategory::SecretLike => BuiltInSecurityRule::Sec2Rule10.id(),
            RiskCategory::EventOrCommandName => BuiltInLiteralRule::Lit1Rule2.id(),
            RiskCategory::RouteOrUrl => BuiltInLiteralRule::Lit1Rule3.id(),
            RiskCategory::ProtocolHeaderOrMedia => BuiltInLiteralRule::Lit1Rule5.id(),
            RiskCategory::RawJsonBlob => BuiltInLiteralRule::Lit1Rule6.id(),
            RiskCategory::SqlFragment => BuiltInLiteralRule::Lit1Rule7.id(),
            RiskCategory::ShellFragment => BuiltInLiteralRule::Lit1Rule8.id(),
            RiskCategory::MagicStringComparison => BuiltInLiteralRule::Lit1Rule4.id(),
            RiskCategory::RepeatedLiteral => BuiltInLiteralRule::Lit1Rule9.id(),
            RiskCategory::IdOrKeyName
            | RiskCategory::StateOrStatus
            | RiskCategory::HumanMessage
            | RiskCategory::TestFixture
            | RiskCategory::ImportSpecifier
            | RiskCategory::SchemaOwnerLiteral
            | RiskCategory::UnknownLiteral => BuiltInLiteralRule::Lit1Rule1.id(),
        }
}

/// One literal-risk result ready for report rendering.
#[derive(Debug, Clone)]
pub struct Finding {
    pub rule_id: RuleId,
    pub severity: Severity,
    pub file: LiteralFindingPath,
    pub line: LiteralSourceLine,
    pub column: LiteralSourceColumn,
    pub language: LiteralLanguageId,
    pub file_role: FileRole,
    pub literal_kind: LiteralKind,
    pub literal_preview: LiteralPreview,
    pub literal_hash: LiteralFindingHash,
    pub category: RiskCategory,
    pub score: LiteralRiskScore,
    pub confidence: LiteralConfidence,
    pub blocking: LiteralFindingDisposition,
    pub reason: LiteralFindingReason,
    pub suggestion: LiteralFindingSuggestion,
    pub context: LiteralSourceContext,
}

/// Counts explaining why discovered files were not scanned.
#[derive(Debug, Clone, Default)]
pub struct IgnoredSummary {
    pub gitignore: LiteralScanCount,
    pub default_dirs: LiteralScanCount,
    pub default_files: LiteralScanCount,
    pub binary: LiteralScanCount,
    pub too_large: LiteralScanCount,
    pub unknown_language: LiteralScanCount,
}

/// Aggregate counts and elapsed time for one literal scan.
#[derive(Debug, Clone)]
pub struct ScanSummary {
    pub files_discovered: LiteralScanCount,
    pub files_scanned: LiteralScanCount,
    pub files_ignored: LiteralScanCount,
    pub literals_found: LiteralScanCount,
    pub literal_risks: LiteralScanCount,
    pub hard_findings: LiteralScanCount,
    pub duration_ms: LiteralScanDurationMillis,
}

/// Complete typed result of one literal scan.
#[derive(Debug, Clone)]
pub struct ScanReport {
    pub ok: ReportOutcome,
    pub summary: ScanSummary,
    pub ignored: IgnoredSummary,
    pub hard_findings: Vec<Finding>,
    pub literal_risks: Vec<Finding>,
    pub languages: BTreeMap<LiteralLanguageId, LiteralScanCount>,
}
