#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::SystemTime;

const DEFAULT_MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const DEFAULT_MIN_SCORE: u8 = 40;

const DEFAULT_IGNORED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    "coverage",
    ".enforce",
    ".ledger",
    "output",
    "test-results",
    "playwright-report",
    ".cache",
    "tmp",
    "temp",
    "logs",
    "log",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".turbo",
    ".vercel",
    ".netlify",
    ".wrangler",
    ".parcel-cache",
    ".vite",
    ".idea",
    ".vscode",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".tox",
    ".venv",
    "venv",
    "env",
    ".eggs",
    ".gradle",
    ".mvn",
    "out",
    "bin",
    "obj",
    "Pods",
    "DerivedData",
];

const DEFAULT_IGNORED_FILE_SUFFIXES: &[&str] = &[
    ".log",
    ".tmp",
    ".temp",
    ".bak",
    ".old",
    ".orig",
    ".swp",
    ".swo",
    ".pid",
    ".min.js",
    ".map",
    ".d.ts.map",
    ".wasm",
    ".png",
    ".jpg",
    ".jpeg",
    ".gif",
    ".webp",
    ".ico",
    ".pdf",
    ".zip",
    ".tar",
    ".gz",
    ".7z",
    ".rar",
    ".mp4",
    ".mov",
    ".mp3",
    ".wav",
    ".sqlite",
    ".db",
    ".duckdb",
];

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileRole {
    Domain,
    Boundary,
    Config,
    Test,
    Generated,
    Tooling,
    Script,
    Docs,
    CommonText,
    Unknown,
}

impl FileRole {
    fn as_str(self) -> &'static str {
        match self {
            FileRole::Domain => "domain",
            FileRole::Boundary => "boundary",
            FileRole::Config => "config",
            FileRole::Test => "test",
            FileRole::Generated => "generated",
            FileRole::Tooling => "tooling",
            FileRole::Script => "script",
            FileRole::Docs => "docs",
            FileRole::CommonText => "common-text",
            FileRole::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LiteralKind {
    Normal,
    Raw,
    Byte,
    Template,
    InterpolatedTemplate,
    Triple,
    FString,
    ImportSpecifier,
    DocString,
    Attribute,
}

impl LiteralKind {
    fn as_str(&self) -> &'static str {
        match self {
            LiteralKind::Normal => "normal",
            LiteralKind::Raw => "raw",
            LiteralKind::Byte => "byte",
            LiteralKind::Template => "template",
            LiteralKind::InterpolatedTemplate => "interpolated-template",
            LiteralKind::Triple => "triple",
            LiteralKind::FString => "f-string",
            LiteralKind::ImportSpecifier => "import-specifier",
            LiteralKind::DocString => "docstring",
            LiteralKind::Attribute => "attribute",
        }
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskCategory {
    SecretLike,
    EventOrCommandName,
    RouteOrUrl,
    ProtocolHeaderOrMedia,
    IdOrKeyName,
    StateOrStatus,
    RawJsonBlob,
    SqlFragment,
    ShellFragment,
    MagicStringComparison,
    RepeatedLiteral,
    HumanMessage,
    TestFixture,
    ImportSpecifier,
    SchemaOwnerLiteral,
    UnknownLiteral,
}

impl RiskCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskCategory::SecretLike => "secret-like",
            RiskCategory::EventOrCommandName => "event-or-command-name",
            RiskCategory::RouteOrUrl => "route-or-url",
            RiskCategory::ProtocolHeaderOrMedia => "protocol-header-or-media",
            RiskCategory::IdOrKeyName => "id-or-key-name",
            RiskCategory::StateOrStatus => "state-or-status",
            RiskCategory::RawJsonBlob => "raw-json-blob",
            RiskCategory::SqlFragment => "sql-fragment",
            RiskCategory::ShellFragment => "shell-fragment",
            RiskCategory::MagicStringComparison => "magic-string-comparison",
            RiskCategory::RepeatedLiteral => "repeated-literal",
            RiskCategory::HumanMessage => "human-message",
            RiskCategory::TestFixture => "test-fixture",
            RiskCategory::ImportSpecifier => "import-specifier",
            RiskCategory::SchemaOwnerLiteral => "schema-owner-literal",
            RiskCategory::UnknownLiteral => "unknown-literal",
        }
    }

    fn rule_id(&self) -> &'static str {
        match self {
            RiskCategory::SecretLike => "SEC-2.10",
            RiskCategory::EventOrCommandName => "LIT-1.2",
            RiskCategory::RouteOrUrl => "LIT-1.3",
            RiskCategory::MagicStringComparison => "LIT-1.4",
            RiskCategory::ProtocolHeaderOrMedia => "LIT-1.5",
            RiskCategory::RawJsonBlob => "LIT-1.6",
            RiskCategory::SqlFragment => "LIT-1.7",
            RiskCategory::ShellFragment => "LIT-1.8",
            RiskCategory::RepeatedLiteral => "LIT-1.9",
            RiskCategory::IdOrKeyName
            | RiskCategory::StateOrStatus
            | RiskCategory::HumanMessage
            | RiskCategory::TestFixture
            | RiskCategory::ImportSpecifier
            | RiskCategory::SchemaOwnerLiteral
            | RiskCategory::UnknownLiteral => "LIT-1.1",
        }
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

impl Finding {
    fn stable_key(&self) -> String {
        format!(
            "{}:{:010}:{:010}:{}:{}:{}",
            self.file,
            self.line,
            self.column,
            self.category.as_str(),
            self.literal_hash,
            self.rule_id
        )
    }

    fn to_json(&self) -> String {
        format!(
            concat!(
                "{{",
                "\"ruleId\":{},",
                "\"severity\":{},",
                "\"file\":{},",
                "\"line\":{},",
                "\"column\":{},",
                "\"language\":{},",
                "\"fileRole\":{},",
                "\"literalKind\":{},",
                "\"literalPreview\":{},",
                "\"literalHash\":{},",
                "\"category\":{},",
                "\"score\":{},",
                "\"confidence\":{},",
                "\"blocking\":{},",
                "\"reason\":{},",
                "\"suggestion\":{},",
                "\"context\":{}",
                "}}"
            ),
            json_string(&self.rule_id),
            json_string(&self.severity),
            json_string(&self.file),
            self.line,
            self.column,
            json_string(&self.language),
            json_string(self.file_role.as_str()),
            json_string(self.literal_kind.as_str()),
            json_string(&self.literal_preview),
            json_string(&self.literal_hash),
            json_string(self.category.as_str()),
            self.score,
            json_string(&self.confidence),
            self.blocking,
            json_string(&self.reason),
            json_string(&self.suggestion),
            json_string(&self.context),
        )
    }
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

impl ScanReport {
    pub fn to_json_pretty(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str(&format!("  \"ok\": {},\n", self.ok));
        out.push_str("  \"summary\": {");
        out.push_str(&format!(
            "\n    \"filesDiscovered\": {},\n    \"filesScanned\": {},\n    \"filesIgnored\": {},\n    \"literalsFound\": {},\n    \"literalRisks\": {},\n    \"hardFindings\": {},\n    \"durationMs\": {}\n  }},\n",
            self.summary.files_discovered,
            self.summary.files_scanned,
            self.summary.files_ignored,
            self.summary.literals_found,
            self.summary.literal_risks,
            self.summary.hard_findings,
            self.summary.duration_ms
        ));
        out.push_str("  \"ignored\": {");
        out.push_str(&format!(
            "\n    \"gitignore\": {},\n    \"defaultDirs\": {},\n    \"defaultFiles\": {},\n    \"binary\": {},\n    \"tooLarge\": {},\n    \"unknownLanguage\": {}\n  }},\n",
            self.ignored.gitignore,
            self.ignored.default_dirs,
            self.ignored.default_files,
            self.ignored.binary,
            self.ignored.too_large,
            self.ignored.unknown_language
        ));
        out.push_str("  \"languages\": {");
        if self.languages.is_empty() {
            out.push_str("},\n");
        } else {
            out.push('\n');
            for (idx, (language, count)) in self.languages.iter().enumerate() {
                let comma = if idx + 1 == self.languages.len() { "" } else { "," };
                out.push_str(&format!("    {}: {}{}\n", json_string(language), count, comma));
            }
            out.push_str("  },\n");
        }
        out.push_str("  \"hardFindings\": [");
        if self.hard_findings.is_empty() {
            out.push_str("],\n");
        } else {
            out.push('\n');
            for (idx, finding) in self.hard_findings.iter().enumerate() {
                let comma = if idx + 1 == self.hard_findings.len() { "" } else { "," };
                out.push_str(&format!("    {}{}\n", finding.to_json(), comma));
            }
            out.push_str("  ],\n");
        }
        out.push_str("  \"literalRisks\": [");
        if self.literal_risks.is_empty() {
            out.push_str("]\n");
        } else {
            out.push('\n');
            for (idx, finding) in self.literal_risks.iter().enumerate() {
                let comma = if idx + 1 == self.literal_risks.len() { "" } else { "," };
                out.push_str(&format!("    {}{}\n", finding.to_json(), comma));
            }
            out.push_str("  ]\n");
        }
        out.push_str("}\n");
        out
    }

    pub fn to_json_lines(&self) -> Vec<String> {
        self.hard_findings
            .iter()
            .chain(self.literal_risks.iter())
            .map(Finding::to_json)
            .collect()
    }

    pub fn to_human(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "Ocentra Literal Scan: {}\nfiles scanned: {}, literals: {}, hard findings: {}, literal risks: {}\n",
            if self.ok { "PASS" } else { "FAIL" },
            self.summary.files_scanned,
            self.summary.literals_found,
            self.summary.hard_findings,
            self.summary.literal_risks
        ));
        for finding in self.hard_findings.iter().chain(self.literal_risks.iter()) {
            out.push_str(&format!(
                "\n{}:{}:{} {} score={} {}\n  {}\n  {}\n  {}\n",
                finding.file,
                finding.line,
                finding.column,
                finding.rule_id,
                finding.score,
                finding.category.as_str(),
                finding.literal_preview,
                finding.reason,
                finding.suggestion
            ));
        }
        out
    }
}

#[derive(Debug, Clone)]
struct FileJob {
    path: PathBuf,
    rel: String,
    language: LanguageSpec,
    role: FileRole,
}

#[derive(Debug, Clone)]
struct FileResult {
    file: String,
    language: String,
    role: FileRole,
    candidates: Vec<LiteralCandidate>,
    findings: Vec<Finding>,
}

pub fn run_scan(opts: &CliOptions) -> io::Result<ScanReport> {
    let started = SystemTime::now();
    let root = match fs::canonicalize(&opts.root) {
        Ok(path) => path,
        Err(_) => opts.root.clone(),
    };
    let language_filter: HashSet<String> = opts.languages.iter().map(|s| s.to_lowercase()).collect();
    let ignore_state = IgnoreState::load(&root, opts.respect_gitignore);
    let mut ignored = IgnoredSummary::default();
    let files = discover_files(&root, opts, &ignore_state, &mut ignored)?;
    let files_discovered = files.len();
    let mut jobs = Vec::new();

    for path in files {
        let rel = normalize_path(path.strip_prefix(&root).unwrap_or(&path));
        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if metadata.len() > opts.max_file_bytes {
            ignored.too_large += 1;
            continue;
        }
        if is_probably_binary(&path)? {
            ignored.binary += 1;
            continue;
        }
        let Some(language) = detect_language(&path, opts.include_unknown_code) else {
            ignored.unknown_language += 1;
            continue;
        };
        if !language_filter.is_empty() && !language_filter.contains(language.id) {
            continue;
        }
        if language.family == LanguageFamily::CommonText || language.family == LanguageFamily::Sql {
            // Not code literal-risk targets. They are left to common/config/security checks in Ocentra Enforcer.
            continue;
        }
        let role = classify_file_role(&rel, language);
        jobs.push(FileJob {
            path,
            rel,
            language,
            role,
        });
    }

    let files_scanned = jobs.len();
    let thread_count = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(4)
        .max(1);
    let chunks = chunk_jobs(jobs, thread_count);
    let mut results = Vec::new();

    thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in chunks {
            handles.push(scope.spawn(move || {
                let mut local = Vec::new();
                for job in chunk {
                    if let Ok(result) = scan_file(job) {
                        local.push(result);
                    }
                }
                local
            }));
        }
        for handle in handles {
            if let Ok(mut local) = handle.join() {
                results.append(&mut local);
            }
        }
    });

    let mut literal_locations: HashMap<String, BTreeSet<String>> = HashMap::new();
    let mut literals_found = 0usize;
    for result in &results {
        literals_found += result.candidates.len();
        for candidate in &result.candidates {
            literal_locations
                .entry(candidate.text.clone())
                .or_default()
                .insert(result.file.clone());
        }
    }

    let mut hard_findings = Vec::new();
    let mut literal_risks = Vec::new();
    let mut languages = BTreeMap::new();

    for mut result in results {
        *languages.entry(result.language.clone()).or_insert(0) += 1;
        for candidate in result.candidates.drain(..) {
            let repeated_files = literal_locations
                .get(&candidate.text)
                .map(BTreeSet::len)
                .unwrap_or(0);
            let finding = classify_literal(
                &candidate,
                &result.file,
                &result.language,
                result.role,
                repeated_files,
                opts.fail_above,
            );
            if finding.category == RiskCategory::ImportSpecifier && !opts.include_low {
                continue;
            }
            if finding.blocking {
                hard_findings.push(finding);
            } else if opts.include_low || finding.score >= opts.min_score {
                literal_risks.push(finding);
            }
        }
        hard_findings.append(&mut result.findings);
    }

    hard_findings.sort_by_key(Finding::stable_key);
    literal_risks.sort_by_key(Finding::stable_key);

    let duration_ms = match started.elapsed() {
        Ok(elapsed) => elapsed.as_millis(),
        Err(_) => 0,
    };
    let files_ignored = ignored.gitignore
        + ignored.default_dirs
        + ignored.default_files
        + ignored.binary
        + ignored.too_large
        + ignored.unknown_language;
    let ok = hard_findings.is_empty();
    Ok(ScanReport {
        ok,
        summary: ScanSummary {
            files_discovered,
            files_scanned,
            files_ignored,
            literals_found,
            literal_risks: literal_risks.len(),
            hard_findings: hard_findings.len(),
            duration_ms,
        },
        ignored,
        hard_findings,
        literal_risks,
        languages,
    })
}

fn scan_file(job: FileJob) -> io::Result<FileResult> {
    let source = fs::read_to_string(&job.path)?;
    let mut candidates = lex_literals(&source, job.language, &job.rel);
    for candidate in &mut candidates {
        if candidate.context.is_empty() {
            candidate.context = match line_at(&source, candidate.line) {
                Some(line) => line,
                None => String::new(),
            };
        }
    }
    Ok(FileResult {
        file: job.rel,
        language: job.language.id.to_string(),
        role: job.role,
        candidates,
        findings: Vec::new(),
    })
}

fn discover_files(
    root: &Path,
    opts: &CliOptions,
    ignore_state: &IgnoreState,
    ignored: &mut IgnoredSummary,
) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let starts = if opts.files.is_empty() {
        vec![root.to_path_buf()]
    } else {
        opts.files
            .iter()
            .map(|entry| {
                if entry.is_absolute() {
                    entry.clone()
                } else {
                    root.join(entry)
                }
            })
            .collect()
    };
    for start in starts {
        walk(root, &start, opts, ignore_state, ignored, &mut out)?;
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn walk(
    root: &Path,
    current: &Path,
    opts: &CliOptions,
    ignore_state: &IgnoreState,
    ignored: &mut IgnoredSummary,
    out: &mut Vec<PathBuf>,
) -> io::Result<()> {
    let metadata = match fs::symlink_metadata(current) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(()),
    };
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    let rel = current.strip_prefix(root).unwrap_or(current);
    let rel_norm = normalize_path(rel);
    if !opts.include_ignored {
        if is_default_ignored_dir(current) && metadata.is_dir() {
            ignored.default_dirs += 1;
            return Ok(());
        }
        if metadata.is_file() && is_default_ignored_file(current) {
            ignored.default_files += 1;
            return Ok(());
        }
        if opts.respect_gitignore && ignore_state.matches(&rel_norm, metadata.is_dir()) {
            ignored.gitignore += 1;
            return Ok(());
        }
    }
    if metadata.is_dir() {
        let mut entries = fs::read_dir(current)?.filter_map(Result::ok).collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.path());
        for entry in entries {
            walk(root, &entry.path(), opts, ignore_state, ignored, out)?;
        }
    } else if metadata.is_file() {
        out.push(current.to_path_buf());
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct IgnoreState {
    patterns: Vec<String>,
}

impl IgnoreState {
    fn load(root: &Path, enabled: bool) -> Self {
        if !enabled {
            return Self { patterns: Vec::new() };
        }
        let mut patterns = Vec::new();
        for rel in [".gitignore", ".ignore", ".git/info/exclude"] {
            let path = root.join(rel);
            if let Ok(text) = fs::read_to_string(path) {
                for line in text.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                        continue;
                    }
                    patterns.push(trimmed.trim_start_matches('/').to_string());
                }
            }
        }
        Self { patterns }
    }

    fn matches(&self, rel: &str, is_dir: bool) -> bool {
        self.patterns.iter().any(|pattern| gitignore_pattern_matches(pattern, rel, is_dir))
    }
}

fn gitignore_pattern_matches(pattern: &str, rel: &str, is_dir: bool) -> bool {
    let mut pat = pattern.trim();
    if pat.is_empty() {
        return false;
    }
    let dir_only = pat.ends_with('/');
    if dir_only {
        pat = pat.trim_end_matches('/');
    }
    if dir_only && !is_dir && !rel.starts_with(&format!("{pat}/")) {
        return false;
    }
    if pat.contains('*') {
        glob_match(pat, rel) || rel.split('/').any(|part| glob_match(pat, part))
    } else if pat.contains('/') {
        rel == pat || rel.starts_with(&format!("{pat}/"))
    } else {
        rel.split('/').any(|part| part == pat)
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    glob_match_bytes(pattern.as_bytes(), text.as_bytes())
}

fn glob_match_bytes(pattern: &[u8], text: &[u8]) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }
    if pattern[0] == b'*' {
        for index in 0..=text.len() {
            if glob_match_bytes(&pattern[1..], &text[index..]) {
                return true;
            }
        }
        return false;
    }
    if !text.is_empty() && (pattern[0] == b'?' || pattern[0] == text[0]) {
        return glob_match_bytes(&pattern[1..], &text[1..]);
    }
    false
}

fn is_default_ignored_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .map(|name| DEFAULT_IGNORED_DIRS.iter().any(|entry| name.eq_ignore_ascii_case(entry)))
        .unwrap_or(false)
}

fn is_default_ignored_file(path: &Path) -> bool {
    let name = path.file_name().and_then(OsStr::to_str).unwrap_or("");
    DEFAULT_IGNORED_FILE_SUFFIXES
        .iter()
        .any(|suffix| name.to_ascii_lowercase().ends_with(&suffix.to_ascii_lowercase()))
}

fn is_probably_binary(path: &Path) -> io::Result<bool> {
    let bytes = fs::read(path)?;
    Ok(bytes.iter().take(4096).any(|byte| *byte == 0))
}

fn chunk_jobs(mut jobs: Vec<FileJob>, chunks: usize) -> Vec<Vec<FileJob>> {
    if jobs.is_empty() {
        return Vec::new();
    }
    jobs.sort_by(|a, b| a.rel.cmp(&b.rel));
    let count = chunks.max(1).min(jobs.len());
    let mut out = vec![Vec::new(); count];
    for (index, job) in jobs.into_iter().enumerate() {
        out[index % count].push(job);
    }
    out
}

pub fn language_registry() -> Vec<LanguageSpec> {
    vec![
        spec("rust", LanguageFamily::Rust, &["rs"], &[], false, false, false),
        spec("typescript", LanguageFamily::TypeScript, &["ts", "tsx", "mts", "cts"], &[], true, true, false),
        spec("javascript", LanguageFamily::TypeScript, &["js", "jsx", "mjs", "cjs"], &[], true, true, false),
        spec("python", LanguageFamily::Python, &["py", "pyw"], &[], true, false, true),
        spec("c", LanguageFamily::CLike, &["c", "h"], &[], false, false, false),
        spec("cpp", LanguageFamily::CLike, &["cc", "cpp", "cxx", "hpp", "hxx", "hh"], &[], false, false, false),
        spec("csharp", LanguageFamily::CLike, &["cs"], &[], true, false, true),
        spec("objective-c", LanguageFamily::CLike, &["m", "mm"], &[], false, false, false),
        spec("zig", LanguageFamily::CLike, &["zig"], &[], false, false, false),
        spec("go", LanguageFamily::CLike, &["go"], &[], false, true, false),
        spec("d", LanguageFamily::CLike, &["d"], &[], false, true, false),
        spec("v", LanguageFamily::CLike, &["v"], &[], false, true, false),
        spec("nim", LanguageFamily::HashComment, &["nim"], &[], true, true, true),
        spec("java", LanguageFamily::CLike, &["java"], &[], false, false, true),
        spec("kotlin", LanguageFamily::CLike, &["kt", "kts"], &[], true, false, true),
        spec("scala", LanguageFamily::CLike, &["scala", "sc"], &[], true, false, true),
        spec("groovy", LanguageFamily::CLike, &["groovy", "gradle"], &[], true, true, true),
        spec("swift", LanguageFamily::CLike, &["swift"], &[], false, false, true),
        spec("dart", LanguageFamily::CLike, &["dart"], &[], true, true, true),
        spec("php", LanguageFamily::CLike, &["php", "phtml"], &[], true, true, false),
        spec("ruby", LanguageFamily::HashComment, &["rb"], &[], true, true, true),
        spec("perl", LanguageFamily::HashComment, &["pl", "pm"], &[], true, true, false),
        spec("lua", LanguageFamily::HashComment, &["lua"], &[], true, false, true),
        spec("r", LanguageFamily::HashComment, &["r", "R"], &[], true, true, false),
        spec("julia", LanguageFamily::HashComment, &["jl"], &[], true, false, true),
        spec("shell", LanguageFamily::Shell, &["sh", "bash", "zsh", "fish"], &[], true, true, false),
        spec("powershell", LanguageFamily::Shell, &["ps1", "psm1", "psd1"], &[], true, true, true),
        spec("batch", LanguageFamily::Shell, &["bat", "cmd"], &[], true, false, false),
        spec("make", LanguageFamily::Shell, &["mk"], &["Makefile", "makefile"], true, true, false),
        spec("dockerfile", LanguageFamily::Shell, &[], &["Dockerfile", "Containerfile"], true, true, false),
        spec("haskell", LanguageFamily::CLike, &["hs", "lhs"], &[], false, false, false),
        spec("ocaml", LanguageFamily::CLike, &["ml", "mli"], &[], true, false, false),
        spec("fsharp", LanguageFamily::CLike, &["fs", "fsx", "fsi"], &[], true, true, true),
        spec("elm", LanguageFamily::CLike, &["elm"], &[], false, false, false),
        spec("purescript", LanguageFamily::CLike, &["purs"], &[], false, false, false),
        spec("elixir", LanguageFamily::HashComment, &["ex", "exs"], &[], true, true, true),
        spec("erlang", LanguageFamily::CLike, &["erl", "hrl"], &[], true, false, false),
        spec("clojure", LanguageFamily::Lisp, &["clj", "cljs", "cljc", "edn"], &[], false, false, false),
        spec("lisp", LanguageFamily::Lisp, &["lisp", "lsp", "scm", "rkt", "el"], &[], false, false, false),
        spec("sql", LanguageFamily::Sql, &["sql", "psql", "mysql", "sqlite"], &[], true, false, false),
        spec("graphql", LanguageFamily::CommonText, &["graphql", "gql"], &[], true, false, false),
        spec("terraform", LanguageFamily::CommonText, &["tf", "tfvars", "hcl"], &[], true, false, false),
        spec("nix", LanguageFamily::CLike, &["nix"], &[], true, false, true),
        spec("starlark", LanguageFamily::HashComment, &["bzl", "star"], &[], true, false, true),
        spec("protobuf", LanguageFamily::CLike, &["proto"], &[], true, false, false),
        spec("thrift", LanguageFamily::CLike, &["thrift"], &[], true, false, false),
        spec("solidity", LanguageFamily::CLike, &["sol"], &[], true, false, false),
        spec("move", LanguageFamily::CLike, &["move"], &[], true, false, false),
        spec("apex", LanguageFamily::CLike, &["cls", "trigger"], &[], true, false, false),
        spec("qml", LanguageFamily::CLike, &["qml"], &[], true, false, false),
        spec("cuda", LanguageFamily::CLike, &["cu", "cuh"], &[], false, false, false),
        spec("shader", LanguageFamily::CLike, &["glsl", "vert", "frag", "geom", "tesc", "tese", "hlsl", "wgsl"], &[], true, false, false),
        spec("raku", LanguageFamily::HashComment, &["raku", "rakumod", "p6", "pm6"], &[], true, true, true),
        spec("reason", LanguageFamily::CLike, &["re", "rei"], &[], true, false, false),
        spec("rescript", LanguageFamily::CLike, &["res", "resi"], &[], true, false, false),
        spec("sml", LanguageFamily::CLike, &["sml", "sig"], &[], true, false, false),
        spec("avro", LanguageFamily::CommonText, &["avsc"], &[], true, false, false),
        spec("html", LanguageFamily::Markup, &["html", "htm", "vue", "svelte", "astro"], &[], true, true, false),
        spec("css", LanguageFamily::CommonText, &["css", "scss", "sass", "less"], &[], true, false, false),
        spec("json", LanguageFamily::CommonText, &["json", "jsonc"], &[], true, false, false),
        spec("yaml", LanguageFamily::CommonText, &["yaml", "yml"], &[], true, false, false),
        spec("toml", LanguageFamily::CommonText, &["toml"], &[], true, false, false),
        spec("env", LanguageFamily::CommonText, &["env"], &[".env", ".env.local", ".env.example"], true, false, false),
        spec("markdown", LanguageFamily::CommonText, &["md", "mdx", "txt"], &[], false, false, false),
        spec("xml", LanguageFamily::CommonText, &["xml"], &[], true, false, false),
        spec("csv", LanguageFamily::CommonText, &["csv"], &[], false, false, false),
    ]
}

fn spec(
    id: &'static str,
    family: LanguageFamily,
    extensions: &'static [&'static str],
    basenames: &'static [&'static str],
    single_quote_strings: bool,
    backtick_strings: bool,
    triple_double_strings: bool,
) -> LanguageSpec {
    LanguageSpec {
        id,
        family,
        extensions,
        basenames,
        single_quote_strings,
        backtick_strings,
        triple_double_strings,
    }
}

fn detect_language(path: &Path, include_unknown: bool) -> Option<LanguageSpec> {
    let registry = language_registry();
    let basename = path.file_name().and_then(OsStr::to_str).unwrap_or("");
    for spec in &registry {
        if spec.basenames.iter().any(|name| *name == basename) {
            return Some(*spec);
        }
    }
    let ext = path.extension().and_then(OsStr::to_str).unwrap_or("");
    for spec in &registry {
        if spec.extensions.iter().any(|candidate| candidate.eq_ignore_ascii_case(ext)) {
            return Some(*spec);
        }
    }
    if include_unknown {
        Some(spec("unknown", LanguageFamily::Fallback, &[], &[], true, true, true))
    } else {
        None
    }
}

fn classify_file_role(rel: &str, language: LanguageSpec) -> FileRole {
    let lower = rel.to_ascii_lowercase();
    if language.family == LanguageFamily::CommonText {
        if lower.ends_with(".md") || lower.ends_with(".mdx") || lower.ends_with(".txt") {
            return FileRole::Docs;
        }
        return FileRole::CommonText;
    }
    if contains_segment(&lower, "generated") || contains_segment(&lower, "__generated__") || lower.contains("auto-generated") {
        return FileRole::Generated;
    }
    if contains_segment(&lower, "test")
        || contains_segment(&lower, "tests")
        || contains_segment(&lower, "__tests__")
        || lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.ends_with("_test.rs")
        || lower.contains("/test_")
    {
        return FileRole::Test;
    }
    if contains_segment(&lower, "boundary")
        || contains_segment(&lower, "boundaries")
        || contains_segment(&lower, "adapter")
        || contains_segment(&lower, "adapters")
        || contains_segment(&lower, "transport")
        || contains_segment(&lower, "serde")
        || contains_segment(&lower, "ffi")
        || contains_segment(&lower, "dto")
        || contains_segment(&lower, "request")
        || contains_segment(&lower, "response")
    {
        return FileRole::Boundary;
    }
    if contains_segment(&lower, "config") || contains_segment(&lower, "settings") || contains_segment(&lower, "env") {
        return FileRole::Config;
    }
    if contains_segment(&lower, "scripts") || contains_segment(&lower, "tools") || language.family == LanguageFamily::Shell {
        return FileRole::Script;
    }
    if contains_segment(&lower, "domain")
        || contains_segment(&lower, "domains")
        || contains_segment(&lower, "core")
        || contains_segment(&lower, "model")
        || contains_segment(&lower, "models")
    {
        return FileRole::Domain;
    }
    FileRole::Unknown
}

fn contains_segment(path: &str, segment: &str) -> bool {
    path.split('/').any(|part| part == segment)
}

fn lex_literals(source: &str, language: LanguageSpec, rel: &str) -> Vec<LiteralCandidate> {
    match language.family {
        LanguageFamily::Rust => lex_rust(source),
        LanguageFamily::TypeScript => lex_c_like(source, language, rel, true),
        LanguageFamily::Python => lex_python(source),
        LanguageFamily::CLike => lex_c_like(source, language, rel, false),
        LanguageFamily::HashComment => lex_hash_comment(source, language),
        LanguageFamily::Shell => lex_shell(source),
        LanguageFamily::Lisp => lex_lisp(source),
        LanguageFamily::Markup => lex_markup(source),
        LanguageFamily::Fallback => lex_c_like(source, language, rel, false),
        LanguageFamily::CommonText | LanguageFamily::Sql => Vec::new(),
    }
}

fn lex_rust(source: &str) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    let mut block_depth = 0usize;
    while index < bytes.len() {
        let ch = bytes[index] as char;
        let next = bytes.get(index + 1).map(|b| *b as char);
        if ch == '\n' {
            line += 1;
            col = 1;
            index += 1;
            continue;
        }
        if block_depth > 0 {
            if ch == '/' && next == Some('*') {
                block_depth += 1;
                index += 2;
                col += 2;
            } else if ch == '*' && next == Some('/') {
                block_depth -= 1;
                index += 2;
                col += 2;
            } else {
                index += 1;
                col += 1;
            }
            continue;
        }
        if ch == '/' && next == Some('/') {
            while index < bytes.len() && bytes[index] as char != '\n' {
                index += 1;
                col += 1;
            }
            continue;
        }
        if ch == '/' && next == Some('*') {
            block_depth = 1;
            index += 2;
            col += 2;
            continue;
        }
        if source.is_char_boundary(index) {
            if let Some((prefix_len, hash_count, kind)) = rust_raw_prefix(&source[index..]) {
            let start_line = line;
            let start_col = col;
            let content_start = index + prefix_len;
            let closing = format!("\"{}", "#".repeat(hash_count));
            if let Some(end_rel) = source[content_start..].find(&closing) {
                let content = &source[content_start..content_start + end_rel];
                out.push(candidate(content, start_line, start_col, kind, line_at(source, start_line)));
                let consumed = prefix_len + end_rel + closing.len();
                advance_position(&source[index..index + consumed], &mut line, &mut col);
                index += consumed;
                continue;
            }
            }
        }
        if ch == 'b' && next == Some('"') {
            if let Some((content, consumed)) = read_quoted(&source[index + 1..], '"') {
                out.push(candidate(&content, line, col, LiteralKind::Byte, line_at(source, line)));
                advance_position(&source[index..index + 1 + consumed], &mut line, &mut col);
                index += 1 + consumed;
                continue;
            }
        }
        if ch == '"' {
            if let Some((content, consumed)) = read_quoted(&source[index..], '"') {
                out.push(candidate(&content, line, col, LiteralKind::Normal, line_at(source, line)));
                advance_position(&source[index..index + consumed], &mut line, &mut col);
                index += consumed;
                continue;
            }
        }
        if ch == '\'' {
            let rest = &source[index..];
            if is_rust_lifetime(rest) {
                index += 1;
                col += 1;
                continue;
            }
            if let Some((_content, consumed)) = read_quoted(rest, '\'') {
                advance_position(&source[index..index + consumed], &mut line, &mut col);
                index += consumed;
                continue;
            }
        }
        index += 1;
        col += 1;
    }
    out
}

fn rust_raw_prefix(rest: &str) -> Option<(usize, usize, LiteralKind)> {
    for prefix in ["br", "r"] {
        if let Some(stripped) = rest.strip_prefix(prefix) {
            let hashes = stripped.chars().take_while(|c| *c == '#').count();
            if stripped.as_bytes().get(hashes) == Some(&b'"') {
                return Some((prefix.len() + hashes + 1, hashes, if prefix == "br" { LiteralKind::Byte } else { LiteralKind::Raw }));
            }
        }
    }
    None
}

fn is_rust_lifetime(rest: &str) -> bool {
    let mut chars = rest.chars();
    chars.next() == Some('\'')
        && chars.next().map(|c| c == '_' || c.is_ascii_alphabetic()).unwrap_or(false)
        && chars.next().map(|c| c.is_ascii_alphanumeric() || c == '_').unwrap_or(false)
}

fn lex_c_like(source: &str, language: LanguageSpec, rel: &str, ts_mode: bool) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    let mut block_comment = false;
    while index < bytes.len() {
        let ch = bytes[index] as char;
        let next = bytes.get(index + 1).map(|b| *b as char);
        if ch == '\n' {
            line += 1;
            col = 1;
            index += 1;
            continue;
        }
        if block_comment {
            if ch == '*' && next == Some('/') {
                block_comment = false;
                index += 2;
                col += 2;
            } else {
                index += 1;
                col += 1;
            }
            continue;
        }
        if ch == '/' && next == Some('/') {
            while index < bytes.len() && bytes[index] as char != '\n' {
                index += 1;
                col += 1;
            }
            continue;
        }
        if ch == '/' && next == Some('*') {
            block_comment = true;
            index += 2;
            col += 2;
            continue;
        }
        if source.is_char_boundary(index) && language.triple_double_strings && source[index..].starts_with("\"\"\"") {
            if let Some(end) = source[index + 3..].find("\"\"\"") {
                let content = &source[index + 3..index + 3 + end];
                out.push(candidate(content, line, col, LiteralKind::Triple, line_at(source, line)));
                let consumed = 3 + end + 3;
                advance_position(&source[index..index + consumed], &mut line, &mut col);
                index += consumed;
                continue;
            }
        }
        if ch == '"' || (language.single_quote_strings && ch == '\'') {
            if let Some((content, consumed)) = read_quoted(&source[index..], ch) {
                let mut kind = LiteralKind::Normal;
                if ts_mode && is_import_specifier_context(line_at(source, line).as_deref().unwrap_or(""), &content) {
                    kind = LiteralKind::ImportSpecifier;
                }
                out.push(candidate(&content, line, col, kind, line_at(source, line)));
                advance_position(&source[index..index + consumed], &mut line, &mut col);
                index += consumed;
                continue;
            }
        }
        if language.backtick_strings && ch == '`' {
            if let Some((content, consumed)) = read_quoted(&source[index..], '`') {
                let kind = if content.contains("${") {
                    LiteralKind::InterpolatedTemplate
                } else {
                    LiteralKind::Template
                };
                out.push(candidate(&content, line, col, kind, line_at(source, line)));
                advance_position(&source[index..index + consumed], &mut line, &mut col);
                index += consumed;
                continue;
            }
        }
        // C# verbatim string @"...".
        if source.is_char_boundary(index) && ch == '@' && next == Some('"') {
            if let Some((content, consumed)) = read_quoted(&source[index + 1..], '"') {
                out.push(candidate(&content, line, col, LiteralKind::Raw, line_at(source, line)));
                advance_position(&source[index..index + 1 + consumed], &mut line, &mut col);
                index += 1 + consumed;
                continue;
            }
        }
        let _ = rel;
        index += 1;
        col += 1;
    }
    out
}

fn lex_python(source: &str) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    let mut last_significant_line_had_block_start = true;
    while index < bytes.len() {
        let ch = bytes[index] as char;
        if ch == '\n' {
            line += 1;
            col = 1;
            index += 1;
            continue;
        }
        if ch == '#' {
            while index < bytes.len() && bytes[index] as char != '\n' {
                index += 1;
                col += 1;
            }
            continue;
        }
        if ch.is_whitespace() {
            index += 1;
            col += 1;
            continue;
        }
        let prefix_len = if source.is_char_boundary(index) { python_string_prefix_len(&source[index..]) } else { 0 };
        let quote_index = index + prefix_len;
        if !source.is_char_boundary(quote_index) {
            index += 1;
            col += 1;
            continue;
        }
        if let Some(quote) = source[quote_index..].chars().next() {
            if quote == '"' || quote == '\'' {
                let start_line = line;
                let start_col = col;
                let is_triple = source[quote_index..].starts_with(&format!("{quote}{quote}{quote}"));
                if is_triple {
                    let delimiter = format!("{quote}{quote}{quote}");
                    if let Some(end) = source[quote_index + 3..].find(&delimiter) {
                        let content = &source[quote_index + 3..quote_index + 3 + end];
                        let mut kind = if prefix_has_f(&source[index..index + prefix_len]) {
                            LiteralKind::FString
                        } else {
                            LiteralKind::Triple
                        };
                        if last_significant_line_had_block_start {
                            kind = LiteralKind::DocString;
                        }
                        out.push(candidate(content, start_line, start_col, kind, line_at(source, start_line)));
                        let consumed = prefix_len + 3 + end + 3;
                        advance_position(&source[index..index + consumed], &mut line, &mut col);
                        index += consumed;
                        last_significant_line_had_block_start = false;
                        continue;
                    }
                } else if let Some((content, consumed_quote)) = read_quoted(&source[quote_index..], quote) {
                    let kind = if prefix_has_f(&source[index..index + prefix_len]) {
                        LiteralKind::FString
                    } else if prefix_len > 0 {
                        LiteralKind::Raw
                    } else {
                        LiteralKind::Normal
                    };
                    out.push(candidate(&content, start_line, start_col, kind, line_at(source, start_line)));
                    let consumed = prefix_len + consumed_quote;
                    advance_position(&source[index..index + consumed], &mut line, &mut col);
                    index += consumed;
                    last_significant_line_had_block_start = false;
                    continue;
                }
            }
        }
        let current_line = match line_at(source, line) {
            Some(line_text) => line_text,
            None => String::new(),
        };
        last_significant_line_had_block_start = current_line.trim_end().ends_with(':');
        index += 1;
        col += 1;
    }
    out
}

fn python_string_prefix_len(rest: &str) -> usize {
    let mut len = 0usize;
    for ch in rest.chars().take(3) {
        if matches!(ch, 'r' | 'R' | 'u' | 'U' | 'b' | 'B' | 'f' | 'F') {
            len += ch.len_utf8();
        } else {
            break;
        }
    }
    let next = rest[len..].chars().next();
    if matches!(next, Some('"') | Some('\'')) {
        len
    } else {
        0
    }
}

fn prefix_has_f(prefix: &str) -> bool {
    prefix.chars().any(|c| c == 'f' || c == 'F')
}

fn lex_hash_comment(source: &str, language: LanguageSpec) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    while index < bytes.len() {
        let ch = bytes[index] as char;
        if ch == '\n' {
            line += 1;
            col = 1;
            index += 1;
            continue;
        }
        if ch == '#' {
            while index < bytes.len() && bytes[index] as char != '\n' {
                index += 1;
                col += 1;
            }
            continue;
        }
        if source.is_char_boundary(index) && language.triple_double_strings && source[index..].starts_with("\"\"\"") {
            if let Some(end) = source[index + 3..].find("\"\"\"") {
                let content = &source[index + 3..index + 3 + end];
                out.push(candidate(content, line, col, LiteralKind::Triple, line_at(source, line)));
                let consumed = 3 + end + 3;
                advance_position(&source[index..index + consumed], &mut line, &mut col);
                index += consumed;
                continue;
            }
        }
        if ch == '"' || (language.single_quote_strings && ch == '\'') {
            if let Some((content, consumed)) = read_quoted(&source[index..], ch) {
                out.push(candidate(&content, line, col, LiteralKind::Normal, line_at(source, line)));
                advance_position(&source[index..index + consumed], &mut line, &mut col);
                index += consumed;
                continue;
            }
        }
        if language.backtick_strings && ch == '`' {
            if let Some((content, consumed)) = read_quoted(&source[index..], '`') {
                out.push(candidate(&content, line, col, LiteralKind::Template, line_at(source, line)));
                advance_position(&source[index..index + consumed], &mut line, &mut col);
                index += consumed;
                continue;
            }
        }
        index += 1;
        col += 1;
    }
    out
}

fn lex_shell(source: &str) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    while index < bytes.len() {
        let ch = bytes[index] as char;
        if ch == '\n' {
            line += 1;
            col = 1;
            index += 1;
            continue;
        }
        if ch == '#' {
            while index < bytes.len() && bytes[index] as char != '\n' {
                index += 1;
                col += 1;
            }
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            if let Some((content, consumed)) = read_quoted(&source[index..], ch) {
                let kind = if ch == '`' { LiteralKind::Template } else { LiteralKind::Normal };
                out.push(candidate(&content, line, col, kind, line_at(source, line)));
                advance_position(&source[index..index + consumed], &mut line, &mut col);
                index += consumed;
                continue;
            }
        }
        index += 1;
        col += 1;
    }
    out
}

fn lex_lisp(source: &str) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    while index < bytes.len() {
        let ch = bytes[index] as char;
        if ch == '\n' {
            line += 1;
            col = 1;
            index += 1;
            continue;
        }
        if ch == ';' {
            while index < bytes.len() && bytes[index] as char != '\n' {
                index += 1;
                col += 1;
            }
            continue;
        }
        if ch == '"' {
            if let Some((content, consumed)) = read_quoted(&source[index..], '"') {
                out.push(candidate(&content, line, col, LiteralKind::Normal, line_at(source, line)));
                advance_position(&source[index..index + consumed], &mut line, &mut col);
                index += consumed;
                continue;
            }
        }
        index += 1;
        col += 1;
    }
    out
}

fn lex_markup(source: &str) -> Vec<LiteralCandidate> {
    let mut out = Vec::new();
    for (line_index, line) in source.lines().enumerate() {
        let mut search = line;
        let mut offset = 0usize;
        while let Some(eq) = search.find('=') {
            let rest = &search[eq + 1..].trim_start();
            let skipped = search[eq + 1..].len() - rest.len();
            if let Some(quote) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') {
                if let Some((content, consumed)) = read_quoted(rest, quote) {
                    out.push(LiteralCandidate {
                        text: content,
                        line: line_index + 1,
                        column: offset + eq + 1 + skipped + 1,
                        kind: LiteralKind::Attribute,
                        context: line.to_string(),
                    });
                    let advance = eq + 1 + skipped + consumed;
                    if advance >= search.len() {
                        break;
                    }
                    offset += advance;
                    search = &search[advance..];
                    continue;
                }
            }
            let advance = eq + 1;
            offset += advance;
            search = &search[advance..];
        }
    }
    out
}

fn read_quoted(source: &str, quote: char) -> Option<(String, usize)> {
    let mut chars = source.char_indices();
    let (_, first) = chars.next()?;
    if first != quote {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    for (idx, ch) in chars {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            out.push(ch);
            continue;
        }
        if ch == quote {
            return Some((out, idx + ch.len_utf8()));
        }
        out.push(ch);
    }
    None
}

fn advance_position(text: &str, line: &mut usize, col: &mut usize) {
    for ch in text.chars() {
        if ch == '\n' {
            *line += 1;
            *col = 1;
        } else {
            *col += 1;
        }
    }
}

fn candidate(text: &str, line: usize, column: usize, kind: LiteralKind, context: Option<String>) -> LiteralCandidate {
    LiteralCandidate {
        text: text.to_string(),
        line,
        column,
        kind,
        context: match context { Some(value) => value, None => String::new() },
    }
}

fn line_at(source: &str, line: usize) -> Option<String> {
    source.lines().nth(line.saturating_sub(1)).map(str::to_string)
}

fn is_import_specifier_context(line: &str, literal: &str) -> bool {
    let trimmed = line.trim_start();
    (trimmed.starts_with("import ")
        || trimmed.starts_with("export ")
        || trimmed.contains("require("))
        && (literal.starts_with('.') || literal.starts_with('/') || !literal.contains(' '))
}

fn classify_literal(
    candidate: &LiteralCandidate,
    file: &str,
    language: &str,
    role: FileRole,
    repeated_files: usize,
    fail_above: Option<u8>,
) -> Finding {
    let mut category = primary_category(candidate, role);
    let mut score = score_literal(candidate, role, &category, repeated_files);
    if repeated_files >= 2 && !matches!(category, RiskCategory::SecretLike | RiskCategory::ImportSpecifier | RiskCategory::TestFixture) {
        category = if score < 70 { RiskCategory::RepeatedLiteral } else { category };
        score = score.saturating_add(if repeated_files >= 3 { 20 } else { 10 }).min(100);
    }
    let blocking = category == RiskCategory::SecretLike || fail_above.map(|threshold| score >= threshold).unwrap_or(false);
    let severity = if blocking {
        "error"
    } else if score >= 70 {
        "warning"
    } else {
        "info"
    };
    let (reason, suggestion) = reason_and_suggestion(&category, role);
    make_finding(
        category.rule_id(),
        severity,
        file,
        candidate,
        language,
        role,
        category,
        score,
        blocking,
        reason,
        suggestion,
    )
}

fn primary_category(candidate: &LiteralCandidate, role: FileRole) -> RiskCategory {
    let text = candidate.text.trim();
    let context = candidate.context.as_str();
    if is_secret_like(text) {
        return RiskCategory::SecretLike;
    }
    if candidate.kind == LiteralKind::ImportSpecifier || is_import_like_context(context, text) {
        return RiskCategory::ImportSpecifier;
    }
    if role == FileRole::Test {
        if is_secret_like(text) {
            return RiskCategory::SecretLike;
        }
        return RiskCategory::TestFixture;
    }
    if is_schema_owner_context(role, context) {
        return RiskCategory::SchemaOwnerLiteral;
    }
    if looks_like_shell(text) {
        return RiskCategory::ShellFragment;
    }
    if looks_like_sql(text) {
        return RiskCategory::SqlFragment;
    }
    if looks_like_json_blob(text) {
        return RiskCategory::RawJsonBlob;
    }
    if looks_like_route_or_url(text) {
        return RiskCategory::RouteOrUrl;
    }
    if looks_like_protocol(text) {
        return RiskCategory::ProtocolHeaderOrMedia;
    }
    if looks_like_event(text) {
        return RiskCategory::EventOrCommandName;
    }
    if is_magic_string_comparison(context) && looks_like_state_or_status(text) {
        return RiskCategory::MagicStringComparison;
    }
    if looks_like_id_or_key(text) {
        return RiskCategory::IdOrKeyName;
    }
    if looks_like_state_or_status(text) {
        return RiskCategory::StateOrStatus;
    }
    if looks_like_human_message(text) {
        return RiskCategory::HumanMessage;
    }
    RiskCategory::UnknownLiteral
}

fn score_literal(candidate: &LiteralCandidate, role: FileRole, category: &RiskCategory, repeated_files: usize) -> u8 {
    let mut score: i16 = 5;
    score += match role {
        FileRole::Domain => 20,
        FileRole::Boundary => 5,
        FileRole::Config => -5,
        FileRole::Test => -30,
        FileRole::Generated => -50,
        FileRole::Tooling | FileRole::Script => -10,
        FileRole::Docs | FileRole::CommonText => -50,
        FileRole::Unknown => 0,
    };
    score += match category {
        RiskCategory::SecretLike => 100,
        RiskCategory::EventOrCommandName => 45,
        RiskCategory::RouteOrUrl => 40,
        RiskCategory::ProtocolHeaderOrMedia => 30,
        RiskCategory::IdOrKeyName => 35,
        RiskCategory::StateOrStatus => 30,
        RiskCategory::RawJsonBlob => 45,
        RiskCategory::SqlFragment => 60,
        RiskCategory::ShellFragment => 70,
        RiskCategory::MagicStringComparison => 45,
        RiskCategory::RepeatedLiteral => 20,
        RiskCategory::HumanMessage => 5,
        RiskCategory::TestFixture => -20,
        RiskCategory::ImportSpecifier => -60,
        RiskCategory::SchemaOwnerLiteral => -30,
        RiskCategory::UnknownLiteral => 0,
    };
    if is_magic_string_comparison(&candidate.context) {
        score += 35;
    }
    if is_logging_context(&candidate.context) {
        score -= 20;
    }
    if repeated_files >= 3 {
        score += 20;
    } else if repeated_files == 2 {
        score += 10;
    }
    score.clamp(0, 100) as u8
}

fn reason_and_suggestion(category: &RiskCategory, role: FileRole) -> (&'static str, &'static str) {
    match category {
        RiskCategory::SecretLike => (
            "Secret-looking string literal found.",
            "Remove the secret from source and rotate exposed credentials.",
        ),
        RiskCategory::EventOrCommandName => (
            "Event or command-like literal found in code.",
            "Move it to an enum, schema constant, generated contract, or protocol owner.",
        ),
        RiskCategory::RouteOrUrl => (
            "Route or URL literal found in code.",
            "Use a route registry, URL newtype, config boundary, or protocol constants.",
        ),
        RiskCategory::ProtocolHeaderOrMedia => (
            "Protocol/header/media literal found.",
            "Prefer a protocol owner constant when this value is repeated or used outside boundary code.",
        ),
        RiskCategory::IdOrKeyName => (
            "ID/key-like literal found.",
            "Use branded keys, schema-owned field names, or generated contract constants.",
        ),
        RiskCategory::StateOrStatus => (
            "State/status-like literal found.",
            "Use an enum, branded union, or state value object rather than magic strings.",
        ),
        RiskCategory::RawJsonBlob => (
            "Raw JSON blob string found in code.",
            "Move JSON into a typed fixture/schema boundary or construct typed values directly.",
        ),
        RiskCategory::SqlFragment => (
            "SQL-like string found in code.",
            "Keep SQL in query owners and parameterize inputs; review for injection risk.",
        ),
        RiskCategory::ShellFragment => (
            "Shell-like command string found in code.",
            "Use argv arrays and reviewed script/tooling boundaries instead of shell strings.",
        ),
        RiskCategory::MagicStringComparison => (
            "String literal appears in comparison/control flow.",
            "Replace with enum/constant/schema value if it encodes domain state.",
        ),
        RiskCategory::RepeatedLiteral => (
            "Repeated string literal found across files.",
            "Move repeated domain/protocol values to one owner constant or generated contract.",
        ),
        RiskCategory::HumanMessage => (
            "Human-readable message literal found.",
            if role == FileRole::Domain {
                "Usually acceptable for display/error text; verify it is not used as domain state."
            } else {
                "Usually acceptable; no action unless repeated or protocol-like."
            },
        ),
        RiskCategory::TestFixture => (
            "Test fixture string literal found.",
            "Usually acceptable; keep secrets and volatile snapshots out of tests.",
        ),
        RiskCategory::ImportSpecifier => (
            "Import/module specifier literal found.",
            "Ignored by literal-risk policy; import-boundary rules should handle architecture.",
        ),
        RiskCategory::SchemaOwnerLiteral => (
            "Literal appears in schema/config/protocol owner context.",
            "Usually acceptable if this file is the declared owner for the value.",
        ),
        RiskCategory::UnknownLiteral => (
            "Unclassified string literal found.",
            "Review only if repeated, used in domain state, or suspicious in context.",
        ),
    }
}

fn make_finding(
    rule_id: &str,
    severity: &str,
    file: &str,
    candidate: &LiteralCandidate,
    language: &str,
    file_role: FileRole,
    category: RiskCategory,
    score: u8,
    blocking: bool,
    reason: &str,
    suggestion: &str,
) -> Finding {
    Finding {
        rule_id: rule_id.to_string(),
        severity: severity.to_string(),
        file: file.to_string(),
        line: candidate.line,
        column: candidate.column,
        language: language.to_string(),
        file_role,
        literal_kind: candidate.kind.clone(),
        literal_preview: preview_literal(&candidate.text, category == RiskCategory::SecretLike),
        literal_hash: format!("fnv128:{}", stable_hash_hex(&candidate.text)),
        category,
        score,
        confidence: if score >= 80 { "high" } else if score >= 50 { "medium" } else { "low" }.to_string(),
        blocking,
        reason: reason.to_string(),
        suggestion: suggestion.to_string(),
        context: candidate.context.trim().chars().take(240).collect(),
    }
}

fn preview_literal(value: &str, redact: bool) -> String {
    if redact {
        return "[REDACTED]".to_string();
    }
    let mut preview = value.chars().take(120).collect::<String>();
    if value.chars().count() > 120 {
        preview.push('…');
    }
    preview
}

fn is_secret_like(text: &str) -> bool {
    let t = text.trim();
    (t.starts_with("ghp_") || t.starts_with("gho_") || t.starts_with("ghu_") || t.starts_with("ghs_") || t.starts_with("ghr_")) && t.len() > 24
        || (t.starts_with("AKIA") && t.len() >= 20 && t.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()))
        || t.starts_with("sk-proj-")
        || t.starts_with("sk-") && t.len() > 24
        || t.starts_with("xoxb-")
        || t.starts_with("xoxp-")
        || t.starts_with("pypi-")
        || t.starts_with("npm_")
        || (t.starts_with("eyJ") && t.matches('.').count() == 2 && t.len() > 40)
        || t.contains("-----BEGIN") && t.contains("PRIVATE KEY-----")
        || ((t.starts_with("sk_live_") || t.starts_with("pk_live_")) && t.len() > 20)
        || looks_high_entropy_secret(t)
}

fn looks_high_entropy_secret(text: &str) -> bool {
    if text.len() < 32 || text.contains(' ') {
        return false;
    }
    let classes = [
        text.chars().any(|c| c.is_ascii_lowercase()),
        text.chars().any(|c| c.is_ascii_uppercase()),
        text.chars().any(|c| c.is_ascii_digit()),
        text.chars().any(|c| matches!(c, '+' | '/' | '_' | '-' | '=' | '.')),
    ];
    classes.iter().filter(|value| **value).count() >= 3
}

fn looks_like_event(text: &str) -> bool {
    let parts: Vec<_> = text.split('.').collect();
    parts.len() >= 2
        && parts.iter().all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        })
}

fn looks_like_route_or_url(text: &str) -> bool {
    text.starts_with("http://")
        || text.starts_with("https://")
        || text.starts_with("ws://")
        || text.starts_with("wss://")
        || (text.starts_with('/') && text.len() > 1 && !text.starts_with("//"))
}

fn looks_like_protocol(text: &str) -> bool {
    matches!(
        text.to_ascii_lowercase().as_str(),
        "get" | "post" | "put" | "patch" | "delete" | "head" | "options" | "content-type" | "authorization" | "accept" | "user-agent" | "application/json" | "text/html" | "application/octet-stream"
    )
}

fn looks_like_id_or_key(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower == "id"
        || lower.ends_with("_id")
        || lower.ends_with("id") && lower.len() > 2 && lower.chars().any(|c| c.is_ascii_uppercase())
        || lower.contains("user_id")
        || lower.contains("device_id")
        || lower.ends_with("_key")
        || lower.ends_with("key") && lower.len() > 3
}

fn looks_like_state_or_status(text: &str) -> bool {
    matches!(
        text.to_ascii_lowercase().as_str(),
        "active" | "inactive" | "enabled" | "disabled" | "ready" | "pending" | "open" | "closed" | "success" | "failure" | "failed" | "running" | "stopped" | "created" | "deleted" | "updated" | "accepted" | "rejected" | "draft" | "published"
    )
}

fn looks_like_json_blob(text: &str) -> bool {
    let trimmed = text.trim();
    ((trimmed.starts_with('{') && trimmed.ends_with('}')) || (trimmed.starts_with('[') && trimmed.ends_with(']')))
        && trimmed.contains(':')
}

fn looks_like_sql(text: &str) -> bool {
    let upper = text.to_ascii_uppercase();
    ["SELECT ", "INSERT ", "UPDATE ", "DELETE ", "CREATE TABLE", "ALTER TABLE", "DROP TABLE"]
        .iter()
        .any(|needle| upper.contains(needle))
}

fn looks_like_shell(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    ["rm -rf", "curl ", "wget ", "cmd /c", "powershell", "invoke-expression", "bash -c", "sh -c", "chmod 777"]
        .iter()
        .any(|needle| lower.contains(needle))
}

fn looks_like_human_message(text: &str) -> bool {
    text.contains(' ')
        && text.len() >= 8
        && !looks_like_sql(text)
        && !looks_like_shell(text)
        && !looks_like_json_blob(text)
}

fn is_import_like_context(context: &str, text: &str) -> bool {
    let trimmed = context.trim_start();
    (trimmed.starts_with("import ")
        || trimmed.starts_with("export ")
        || trimmed.starts_with("use ")
        || trimmed.contains("require("))
        && (text.starts_with('.') || text.starts_with('/') || !text.contains(' '))
}

fn is_schema_owner_context(role: FileRole, context: &str) -> bool {
    role == FileRole::Config || context.contains("Schema") || context.contains("schema") || context.contains("defineLiteral")
}

fn is_magic_string_comparison(context: &str) -> bool {
    context.contains("==")
        || context.contains("===")
        || context.contains("!=")
        || context.contains("!==")
        || context.trim_start().starts_with("case ")
        || context.contains("match ")
        || context.contains("switch")
}

fn is_logging_context(context: &str) -> bool {
    let lower = context.to_ascii_lowercase();
    lower.contains("println")
        || lower.contains("log")
        || lower.contains("tracing::")
        || lower.contains("console.")
        || lower.contains("logger")
}

fn json_string(value: impl AsRef<str>) -> String {
    let value = value.as_ref();
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn normalize_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn stable_hash_hex(text: &str) -> String {
    // Deterministic non-cryptographic 128-bit FNV-1a pair. Do not use for security.
    let mut a: u64 = 0xcbf29ce484222325;
    let mut b: u64 = 0x84222325cbf29ce4;
    for byte in text.as_bytes() {
        a ^= u64::from(*byte);
        a = a.wrapping_mul(0x100000001b3);
        b ^= u64::from(*byte).rotate_left(1);
        b = b.wrapping_mul(0x100000001b3);
    }
    format!("{a:016x}{b:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn rust_lexer_skips_comments_chars_and_lifetimes() {
        let source = r##"
// "comment"
let c = 'x';
let s = "device.connected";
let raw = r#"/api/devices"#;
fn f<'a>(x: &'a str) {}
"##;
        let literals = lex_rust(source);
        let texts = literals.iter().map(|lit| lit.text.as_str()).collect::<Vec<_>>();
        assert!(texts.contains(&"device.connected"));
        assert!(texts.contains(&"/api/devices"));
        assert!(!texts.contains(&"comment"));
        assert!(!texts.contains(&"x"));
    }

    #[test]
    fn ts_lexer_classifies_import_specifier() {
        let spec = detect_language(Path::new("x.ts"), false).unwrap();
        let literals = lex_c_like("import x from './x';\nconst s = 'active';", spec, "x.ts", true);
        assert_eq!(literals[0].kind, LiteralKind::ImportSpecifier);
        assert_eq!(literals[1].text, "active");
    }

    #[test]
    fn python_lexer_extracts_fstrings_and_docstrings() {
        let source = "\"\"\"module doc\"\"\"\nvalue = f'user.{kind}'\n# 'comment'\n";
        let literals = lex_python(source);
        assert!(literals.iter().any(|lit| lit.kind == LiteralKind::DocString));
        assert!(literals.iter().any(|lit| lit.kind == LiteralKind::FString));
        assert!(!literals.iter().any(|lit| lit.text == "comment"));
    }

    #[test]
    fn scoring_marks_domain_event_high_and_test_fixture_low() {
        let domain = LiteralCandidate {
            text: "device.connected".to_string(),
            line: 1,
            column: 1,
            kind: LiteralKind::Normal,
            context: "let x = \"device.connected\";".to_string(),
        };
        let risk = classify_literal(&domain, "src/domain/events.rs", "rust", FileRole::Domain, 1, None);
        assert!(risk.score >= 70, "expected high risk, got {}", risk.score);
        assert_eq!(risk.category, RiskCategory::EventOrCommandName);

        let test = classify_literal(&domain, "tests/events.test.ts", "typescript", FileRole::Test, 1, None);
        assert!(test.score < risk.score);
        assert_eq!(test.category, RiskCategory::TestFixture);
    }

    #[test]
    fn secret_is_blocking() {
        let candidate = LiteralCandidate {
            text: "sk-proj-abcdefghijklmnopqrstuvwxyz123456".to_string(),
            line: 1,
            column: 1,
            kind: LiteralKind::Normal,
            context: "const key = \"sk-proj-...\";".to_string(),
        };
        let finding = classify_literal(&candidate, "src/config.ts", "typescript", FileRole::Config, 1, None);
        assert_eq!(finding.category, RiskCategory::SecretLike);
        assert!(finding.blocking);
        assert_eq!(finding.literal_preview, "[REDACTED]");
    }

    #[test]
    fn markdown_is_common_text_not_code() {
        let spec = detect_language(Path::new("README.md"), false).unwrap();
        assert_eq!(spec.family, LanguageFamily::CommonText);
    }

    #[test]
    fn gitignore_and_default_ignored_dirs_work() {
        let root = temp_dir("literal_scan_ignore");
        fs::create_dir_all(root.join("dist")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join(".gitignore"), "ignored.rs\n").unwrap();
        fs::write(root.join("dist/bad.rs"), "let x = \"device.connected\";").unwrap();
        fs::write(root.join("ignored.rs"), "let x = \"device.connected\";").unwrap();
        fs::write(root.join("src/good.rs"), "let x = \"device.connected\";").unwrap();
        let opts = CliOptions { root: root.clone(), include_low: true, ..CliOptions::default() };
        let report = run_scan(&opts).unwrap();
        assert_eq!(report.summary.files_scanned, 1);
        assert!(report.ignored.default_dirs >= 1);
        assert!(report.ignored.gitignore >= 1);
        let _ = fs::remove_dir_all(root);
    }

    fn temp_dir(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("{name}_{nanos}"));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
