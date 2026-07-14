use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::{DEFAULT_MAX_FILE_BYTES, DEFAULT_MIN_SCORE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    JsonLines,
    Human,
}

#[derive(Debug, Clone)]
pub struct CliOptions {
    pub command: String,
    pub root: PathBuf,
    pub files: Vec<PathBuf>,
    pub output_format: OutputFormat,
    pub min_score: u8,
    pub include_low: bool,
    pub include_ignored: bool,
    pub include_unknown_code: bool,
    pub respect_gitignore: bool,
    pub max_file_bytes: u64,
    pub fail_above: Option<u8>,
    pub languages: Vec<String>,
    pub explain_category: Option<String>,
    pub help: bool,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            command: "scan".to_string(),
            root: PathBuf::from("."),
            files: Vec::new(),
            output_format: OutputFormat::Json,
            min_score: DEFAULT_MIN_SCORE,
            include_low: false,
            include_ignored: false,
            include_unknown_code: false,
            respect_gitignore: true,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            fail_above: None,
            languages: Vec::new(),
            explain_category: None,
            help: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LanguageFamily {
    Rust,
    TypeScript,
    Python,
    CLike,
    HashComment,
    Shell,
    Lisp,
    Markup,
    CommonText,
    Sql,
    Fallback,
}

#[derive(Debug, Clone, Copy)]
pub struct LanguageSpec {
    pub id: &'static str,
    pub family: LanguageFamily,
    pub extensions: &'static [&'static str],
    pub basenames: &'static [&'static str],
    pub single_quote_strings: bool,
    pub backtick_strings: bool,
    pub triple_double_strings: bool,
}

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileRole {
    Domain = 0,
    Boundary = 1,
    Config = 2,
    Test = 3,
    Generated = 4,
    Tooling = 5,
    Script = 6,
    Docs = 7,
    CommonText = 8,
    Unknown = 9,
}

const FILE_ROLE_NAMES: [&str; 10] = [
    "domain",
    "boundary",
    "config",
    "test",
    "generated",
    "tooling",
    "script",
    "docs",
    "common-text",
    "unknown",
];

impl FileRole {
    pub(crate) fn as_str(self) -> &'static str {
        FILE_ROLE_NAMES
            .get(self as usize)
            .copied()
            .unwrap_or("unknown")
    }
}

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LiteralKind {
    Normal = 0,
    Raw = 1,
    Byte = 2,
    Template = 3,
    InterpolatedTemplate = 4,
    Triple = 5,
    FString = 6,
    ImportSpecifier = 7,
    DocString = 8,
    Attribute = 9,
}

const LITERAL_KIND_NAMES: [&str; 10] = [
    "normal",
    "raw",
    "byte",
    "template",
    "interpolated-template",
    "triple",
    "f-string",
    "import-specifier",
    "docstring",
    "attribute",
];

impl LiteralKind {
    pub(crate) fn as_str(&self) -> &'static str {
        LITERAL_KIND_NAMES
            .get(*self as usize)
            .copied()
            .unwrap_or("normal")
    }
}

#[derive(Debug, Clone)]
pub struct LiteralCandidate {
    pub text: String,
    pub line: usize,
    pub column: usize,
    pub kind: LiteralKind,
    pub context: String,
}

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskCategory {
    SecretLike = 0,
    EventOrCommandName = 1,
    RouteOrUrl = 2,
    ProtocolHeaderOrMedia = 3,
    IdOrKeyName = 4,
    StateOrStatus = 5,
    RawJsonBlob = 6,
    SqlFragment = 7,
    ShellFragment = 8,
    MagicStringComparison = 9,
    RepeatedLiteral = 10,
    HumanMessage = 11,
    TestFixture = 12,
    ImportSpecifier = 13,
    SchemaOwnerLiteral = 14,
    UnknownLiteral = 15,
}

const RISK_CATEGORY_NAMES: [&str; 16] = [
    "secret-like",
    "event-or-command-name",
    "route-or-url",
    "protocol-header-or-media",
    "id-or-key-name",
    "state-or-status",
    "raw-json-blob",
    "sql-fragment",
    "shell-fragment",
    "magic-string-comparison",
    "repeated-literal",
    "human-message",
    "test-fixture",
    "import-specifier",
    "schema-owner-literal",
    "unknown-literal",
];

const RISK_CATEGORY_RULE_IDS: [&str; 16] = [
    "SEC-2.10", "LIT-1.2", "LIT-1.3", "LIT-1.5", "LIT-1.1", "LIT-1.1", "LIT-1.6", "LIT-1.7",
    "LIT-1.8", "LIT-1.4", "LIT-1.9", "LIT-1.1", "LIT-1.1", "LIT-1.1", "LIT-1.1", "LIT-1.1",
];

impl RiskCategory {
    pub fn as_str(&self) -> &'static str {
        RISK_CATEGORY_NAMES
            .get(*self as usize)
            .copied()
            .unwrap_or("unknown-literal")
    }

    pub(crate) fn rule_id(&self) -> &'static str {
        RISK_CATEGORY_RULE_IDS
            .get(*self as usize)
            .copied()
            .unwrap_or("LIT-1.1")
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub rule_id: String,
    pub severity: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub language: String,
    pub file_role: FileRole,
    pub literal_kind: LiteralKind,
    pub literal_preview: String,
    pub literal_hash: String,
    pub category: RiskCategory,
    pub score: u8,
    pub confidence: String,
    pub blocking: bool,
    pub reason: String,
    pub suggestion: String,
    pub context: String,
}

#[derive(Debug, Clone, Default)]
pub struct IgnoredSummary {
    pub gitignore: usize,
    pub default_dirs: usize,
    pub default_files: usize,
    pub binary: usize,
    pub too_large: usize,
    pub unknown_language: usize,
}

#[derive(Debug, Clone)]
pub struct ScanSummary {
    pub files_discovered: usize,
    pub files_scanned: usize,
    pub files_ignored: usize,
    pub literals_found: usize,
    pub literal_risks: usize,
    pub hard_findings: usize,
    pub duration_ms: u128,
}

#[derive(Debug, Clone)]
pub struct ScanReport {
    pub ok: bool,
    pub summary: ScanSummary,
    pub ignored: IgnoredSummary,
    pub hard_findings: Vec<Finding>,
    pub literal_risks: Vec<Finding>,
    pub languages: BTreeMap<String, usize>,
}
