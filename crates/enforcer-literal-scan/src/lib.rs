#![forbid(unsafe_code)]
// Inherited from the standalone Tools/ocentra-literal-scan tool (arc-13
// fold-in preserves lexer/scoring behavior as-is). The lexer family shares
// a cursor-state signature (`source`/`out`/`index`/`line`/`col`/...)
// across ~10 call sites; a param-struct refactor is out of scope for this
// fold-in workpack ("preserve existing scanner behavior -- no regression"
// per docs/plans/enforcer-selfhost-plan/workpacks/arc-13-enforcer-literal-scan.md).
#![allow(clippy::too_many_arguments)]

#[path = "discovery-ignore.rs"]
mod discovery_ignore;
mod discovery_ignore_binary;
mod discovery_ignore_filter;
mod discovery_ignore_glob;
mod discovery_ignore_load;
mod discovery_ignore_match;
mod discovery_ignore_state;
mod discovery_ignore_walk;
#[path = "file-role.rs"]
mod file_role;
#[path = "language-registry.rs"]
mod language_registry;
#[path = "lexer-c-like.rs"]
mod lexer_c_like;
mod lexer_c_like_scan;
mod lexer_c_like_string;
mod lexer_hash_comment_scan;
mod lexer_import_context;
mod lexer_lisp_scan;
mod lexer_markup_attr;
mod lexer_markup_scan;
#[path = "lexer-python.rs"]
mod lexer_python;
mod lexer_python_prefix;
mod lexer_python_scan;
mod lexer_python_string;
#[path = "lexer-rust.rs"]
mod lexer_rust;
mod lexer_rust_helpers;
mod lexer_rust_scan;
mod lexer_rust_string;
#[path = "lexer-shared.rs"]
mod lexer_shared;
mod lexer_shell_scan;
#[path = "risk-heuristics.rs"]
mod risk_heuristics;

mod discovery;
mod languages;
mod lexer;
mod models;
mod report_output;
mod risk;
mod risk_finding;
mod risk_heuristics_context;
mod risk_heuristics_literals;
mod risk_heuristics_secret;
mod risk_primary;
mod risk_primary_patterns;
mod risk_primary_roles;
mod risk_reason;
mod risk_score;
mod scan_jobs;
mod scan_parallel;
mod scan_results;
mod scan_runtime;
mod scan_types;
mod utils;

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

pub use languages::language_registry;
pub use models::{
    CliOptions, FileRole, Finding, IgnoredSummary, LanguageFamily, LanguageSpec, LiteralCandidate,
    LiteralKind, OutputFormat, RiskCategory, ScanReport, ScanSummary,
};
pub use scan_runtime::run_scan;

pub(crate) use discovery::scan_file;
pub(crate) use languages::{classify_file_role, detect_language};
pub(crate) use scan_types::{FileJob, FileResult};
pub(crate) use utils::{normalize_path, stable_hash_hex};

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod lib_tests;
