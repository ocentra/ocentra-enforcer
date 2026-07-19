#![forbid(unsafe_code)]
#[path = "boundary/discovery_ignore_binary.rs"]
mod discovery_ignore_binary;
#[path = "boundary/discovery_ignore_filter.rs"]
mod discovery_ignore_filter;
#[path = "boundary/discovery_ignore_glob.rs"]
mod discovery_ignore_glob;
#[path = "boundary/discovery_ignore_load.rs"]
mod discovery_ignore_load;
#[path = "boundary/discovery_ignore_match.rs"]
mod discovery_ignore_match;
#[path = "boundary/discovery_ignore_state.rs"]
mod discovery_ignore_state;
#[path = "boundary/discovery_ignore_walk.rs"]
mod discovery_ignore_walk;
#[path = "boundary/file-role.rs"]
mod file_role;
#[path = "boundary/language-registry.rs"]
mod language_registry;
#[path = "boundary/lexer_c_like_scan.rs"]
mod lexer_c_like_scan;
#[path = "boundary/lexer_c_like_string.rs"]
mod lexer_c_like_string;
#[path = "boundary/lexer_hash_comment_scan.rs"]
mod lexer_hash_comment_scan;
#[path = "boundary/lexer_import_context.rs"]
mod lexer_import_context;
#[path = "boundary/lexer_lisp_scan.rs"]
mod lexer_lisp_scan;
#[path = "boundary/lexer_markup_attr.rs"]
mod lexer_markup_attr;
#[path = "boundary/lexer_markup_scan.rs"]
mod lexer_markup_scan;
#[path = "boundary/lexer_python_prefix.rs"]
mod lexer_python_prefix;
#[path = "boundary/lexer_python_scan.rs"]
mod lexer_python_scan;
#[path = "boundary/lexer_python_string.rs"]
mod lexer_python_string;
#[path = "boundary/lexer_rust_helpers.rs"]
mod lexer_rust_helpers;
#[path = "boundary/lexer_rust_scan.rs"]
mod lexer_rust_scan;
#[path = "boundary/lexer_rust_string.rs"]
mod lexer_rust_string;
#[path = "boundary/lexer-shared.rs"]
mod lexer_shared;
#[path = "boundary/lexer_shell_scan.rs"]
mod lexer_shell_scan;

#[path = "boundary/bridge.rs"]
pub mod bridge;
#[path = "boundary/discovery.rs"]
mod discovery;
#[path = "boundary/json_wire.rs"]
mod json_wire;
#[path = "boundary/lexer.rs"]
mod lexer;
#[path = "boundary/path_normalization.rs"]
mod path_normalization;
#[path = "boundary/report_output.rs"]
mod report_output;
#[path = "boundary/risk.rs"]
mod risk;
#[path = "boundary/risk_finding.rs"]
mod risk_finding;
#[path = "boundary/risk_heuristics_context.rs"]
mod risk_heuristics_context;
#[path = "boundary/risk_heuristics_literals.rs"]
mod risk_heuristics_literals;
#[path = "boundary/risk_heuristics_secret.rs"]
mod risk_heuristics_secret;
#[path = "boundary/risk_primary.rs"]
mod risk_primary;
#[path = "boundary/risk_primary_patterns.rs"]
mod risk_primary_patterns;
#[path = "boundary/risk_primary_roles.rs"]
mod risk_primary_roles;
#[path = "boundary/risk_reason.rs"]
mod risk_reason;
#[path = "boundary/risk_score.rs"]
mod risk_score;
#[path = "boundary/scan_jobs.rs"]
mod scan_jobs;
#[path = "boundary/scan_parallel.rs"]
mod scan_parallel;
#[path = "boundary/scan_results.rs"]
mod scan_results;
#[path = "boundary/scan_types.rs"]
mod scan_types;
#[path = "boundary/stable_hash.rs"]
mod stable_hash;

include!("languages.rs");
include!("models.rs");
include!("boundary/scan_runtime.rs");

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

#[cfg(test)]
mod lib_tests;
