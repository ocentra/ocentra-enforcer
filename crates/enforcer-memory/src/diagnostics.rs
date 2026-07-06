//! X06.7: structured diagnostics for the MCP/CLI/watch surface.
//!
//! # stdout purity (hard requirement)
//!
//! Per the baseline schema doc §1 ("structured KV logs to stderr ONLY --
//! stdout reserved for JSON-RPC") and the workpack's own "no raw
//! prompt/private source text in diagnostics" requirement: this module
//! NEVER writes to stdout, and every line it produces is redacted before
//! emission (see [`redact`]). [`Diagnostics::emit`] is the only function
//! in this crate that performs a `print_stderr`-shaped write, and it does
//! so through a caller-supplied `Write` sink in tests / a real `stderr()`
//! handle in [`emit_to_stderr`] -- kept as the ONE narrow, documented
//! `#[allow(clippy::print_stderr)]` boundary in this module, mirroring
//! `enforcer_mcp::sink`'s "one sanctioned sink module" pattern rather than
//! scattering the allow across the crate.
//!
//! # Levels and formats (binding shape:
//! `refs/x06-baseline-tool-schemas.md` §1)
//!
//! The baseline's own stderr contract (`CBM_LOG_LEVEL`/`CBM_LOG_FORMAT`,
//! `src/foundation/log.c`) is mirrored here with this crate's own env var
//! names: [`Level`] is read from `ENFORCER_MEMORY_LOG_LEVEL`
//! (`debug|info|warn|error|none`, case-insensitive, OR the numeric forms
//! `0`-`4` in that same order, matching `CBM_LOG_DEBUG=0..CBM_LOG_NONE=4`;
//! defaulting to [`Level::Info`] on anything unrecognized or unset --
//! `none` disables all logging, matching the baseline's `CBM_LOG_NONE`);
//! the output [`Format`] is read from `ENFORCER_MEMORY_LOG_FORMAT`
//! (`text`/`json`, defaulting to [`Format::Text`]). Both are read fresh on
//! every [`Diagnostics::from_env`] call rather than cached, so tests can
//! change the environment and observe the effect without process restart.
//!
//! Line shape matches the baseline's two formats exactly (field NAMES
//! differ per this crate's own record shapes, but the `msg=`/`event`
//! framing key and log-line structure are ported 1:1):
//! - **text**: `level=<lvl> msg=<event> key1=val1 key2=val2 ...`
//! - **json**: `{"level":"<lvl>","event":"<event>",...}`
//!
//! # Record kinds
//!
//! - [`RequestRecord`] -- one per MCP/CLI tool invocation: protocol,
//!   method, tool name, duration, error flag (never the request/response
//!   payload itself).
//! - [`FileSkipRecord`] -- one per file the indexer could not process:
//!   path, reason, phase. Per the workpack's "never silent skip, never
//!   fail the run" doctrine, a skip is always recorded, never dropped.

use std::fmt;
use std::io::Write;
use std::time::Duration;

/// Log severity. Ordinal order matches the baseline's `CBM_LOG_*`
/// constants exactly (`debug=0, info=1, warn=2, error=3, none=4`, binding:
/// `refs/x06-baseline-tool-schemas.md` §1) so the numeric env-var forms
/// this crate accepts (see [`Level::from_env_str`]) resolve to the same
/// level a baseline-familiar operator would expect from that number.
/// [`Level::None`] is a configured-minimum-only sentinel (never a record's
/// own severity) that disables all logging, matching `CBM_LOG_NONE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
    /// Disables all logging when configured as the minimum. Never used as
    /// a record's own emitted severity.
    None,
}

impl Level {
    /// Parse one `ENFORCER_MEMORY_LOG_LEVEL` value (name or baseline
    /// numeric form) into a [`Level`]. `pub` (rather than crate-private)
    /// solely so `tests/unit_diagnostics.rs` -- an external compilation
    /// unit per this crate's "no inline `#[cfg(test)]` modules" style --
    /// can exercise this parsing table directly; [`Diagnostics::from_env`]
    /// remains the one real call site.
    pub fn from_env_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "0" | "debug" => Some(Level::Debug),
            "1" | "info" => Some(Level::Info),
            "2" | "warn" | "warning" => Some(Level::Warn),
            "3" | "error" => Some(Level::Error),
            "4" | "none" => Some(Level::None),
            _ => None,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
            Level::None => "none",
        }
    }

    /// Whether a record AT `self` severity should be emitted when the
    /// configured minimum is `configured_min`. Ordinal order is
    /// least-to-most-severe (`Debug` = 0 .. `None` = 4, matching the
    /// baseline's `CBM_LOG_*` constants), so a record is emitted iff its
    /// ordinal is >= the configured minimum's ordinal -- mirroring the
    /// baseline's own gate exactly (`log.c:209-211`: `if (level <
    /// g_log_level) return;`, i.e. a call below the configured level is a
    /// no-op). Configuring [`Level::None`] as the minimum suppresses
    /// every record, including one emitted AT [`Level::None`] (which no
    /// caller in this crate ever does -- see the type's own doc). `pub`
    /// for the same reason as [`Level::from_env_str`]: `tests/unit_diagnostics.rs`
    /// exercises this gating predicate directly as an external compilation unit.
    pub fn should_emit(self, configured_min: Level) -> bool {
        self >= configured_min
    }
}

/// Output encoding for emitted diagnostic lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// `key=value key2=value2 ...`, one line, values quoted if they
    /// contain whitespace.
    Text,
    /// One-line JSON object per record.
    Json,
}

/// One diagnostics sink, configured once from the environment (or
/// explicitly, for tests) and reused for every [`Diagnostics::emit`] call
/// in a process/session.
#[derive(Debug, Clone, Copy)]
pub struct Diagnostics {
    pub level: Level,
    pub format: Format,
}

impl Diagnostics {
    /// Explicit construction (primarily for tests that want a fixed
    /// level/format regardless of the process environment).
    pub fn new(level: Level, format: Format) -> Self {
        Self { level, format }
    }

    /// Read `ENFORCER_MEMORY_LOG_LEVEL`/`ENFORCER_MEMORY_LOG_FORMAT` from
    /// the process environment, defaulting to [`Level::Info`]/
    /// [`Format::Text`] for anything unset or unrecognized.
    pub fn from_env() -> Self {
        let level = std::env::var("ENFORCER_MEMORY_LOG_LEVEL")
            .ok()
            .and_then(|value| Level::from_env_str(&value))
            .unwrap_or(Level::Info);
        let format = std::env::var("ENFORCER_MEMORY_LOG_FORMAT")
            .ok()
            .map(|value| value.to_ascii_lowercase())
            .and_then(|value| match value.as_str() {
                "json" => Some(Format::Json),
                "text" => Some(Format::Text),
                _ => None,
            })
            .unwrap_or(Format::Text);
        Self { level, format }
    }

    /// Emit `record` to `sink` if its level clears the configured
    /// minimum. Every field is already redacted by the record's own
    /// `fields` (see [`redact`]) before this ever formats a line, so this
    /// function itself performs no additional filtering of content --
    /// only of whether the line is emitted at all.
    pub fn emit(
        &self,
        sink: &mut impl Write,
        level: Level,
        record: &impl Record,
    ) -> std::io::Result<()> {
        if !level.should_emit(self.level) {
            return Ok(());
        }
        let line = format_record(level, record, self.format);
        writeln!(sink, "{line}")
    }
}

/// A diagnostics record: anything that can render itself as an ordered
/// list of redacted `(key, value)` fields. Implemented by
/// [`RequestRecord`] and [`FileSkipRecord`]; kept as a trait (rather than
/// a fixed enum) so a future record kind can be added without touching
/// this module's formatting code.
pub trait Record {
    /// The record's fixed event name (e.g. `"mcp.request"`,
    /// `"file_skip"`) -- emitted as `msg=` (text format) / `"event"`
    /// (json format), matching the baseline's own `msg=<event>` /
    /// `{"event":...}` framing key (binding:
    /// `refs/x06-baseline-tool-schemas.md` §1).
    fn event(&self) -> &'static str;
    /// Ordered `(key, value)` pairs, values ALREADY passed through
    /// [`redact`] by the implementor -- this trait does not re-redact,
    /// so every implementor is individually responsible (both this
    /// module's two record types satisfy it; the redaction test covers
    /// both).
    fn fields(&self) -> Vec<(&'static str, String)>;
}

/// One per-request diagnostic: which transport served the call, which
/// JSON-RPC method, which tool (if any -- `initialize`/`ping`/
/// `tools/list` have none), success/failure, and how long it took.
/// Deliberately excludes the request arguments and the result payload --
/// only fixed enum-like names and numeric/boolean metadata are recorded,
/// so this record type can never leak source text even if a future field
/// is added carelessly (the fixed field list below is exhaustive, not a
/// passthrough of caller data).
#[derive(Debug, Clone, PartialEq)]
pub struct RequestRecord {
    /// `"mcp"` or `"cli"` -- which transport served this request.
    pub protocol: &'static str,
    /// The JSON-RPC method (`"tools/call"`, `"initialize"`, ...) or, for
    /// the CLI transport, the CLI's own invocation shape (`"cli"`).
    pub method: String,
    /// The tool name, when this request was a `tools/call`/CLI tool
    /// invocation; `None` for `initialize`/`ping`/`tools/list`.
    pub tool: Option<String>,
    pub duration: Duration,
    pub is_error: bool,
}

impl RequestRecord {
    /// The level this record should be emitted at: WARN on error, INFO
    /// otherwise (binding spec).
    pub fn level(&self) -> Level {
        if self.is_error {
            Level::Warn
        } else {
            Level::Info
        }
    }
}

impl Record for RequestRecord {
    fn event(&self) -> &'static str {
        "mcp.request"
    }

    fn fields(&self) -> Vec<(&'static str, String)> {
        let status = if self.is_error { "error" } else { "ok" };
        vec![
            ("protocol", self.protocol.to_owned()),
            ("method", redact(&self.method)),
            ("tool", self.tool.as_deref().map(redact).unwrap_or_default()),
            ("status", status.to_owned()),
            ("durationMs", self.duration.as_millis().to_string()),
        ]
    }
}

/// Which indexing phase a [`FileSkipRecord`] occurred in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipPhase {
    Walk,
    Parse,
    Extract,
}

impl fmt::Display for SkipPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SkipPhase::Walk => "walk",
            SkipPhase::Parse => "parse",
            SkipPhase::Extract => "extract",
        };
        f.write_str(s)
    }
}

/// One per-file skip diagnostic: path, human reason, and which phase
/// skipped it. Per the workpack's "never silent skip" doctrine, this is
/// the record type callers emit instead of dropping a file with no
/// trace. The `path` is the repo-relative path only (never file
/// CONTENTS), and [`redact`] still runs over both string fields as a
/// defense-in-depth measure against a `reason` string that was built by
/// interpolating file content by mistake.
#[derive(Debug, Clone, PartialEq)]
pub struct FileSkipRecord {
    pub path: String,
    pub reason: String,
    pub phase: SkipPhase,
}

impl Record for FileSkipRecord {
    fn event(&self) -> &'static str {
        "file_skip"
    }

    fn fields(&self) -> Vec<(&'static str, String)> {
        vec![
            ("path", redact(&self.path)),
            ("reason", redact_free_text(&self.reason)),
            ("phase", self.phase.to_string()),
        ]
    }
}

/// Redact a diagnostic field value before it is ever formatted into a
/// line. Two defenses, matching the workpack's "no raw prompt/private
/// source text in diagnostics" requirement literally:
///
/// 1. hard length cap -- a value longer than [`MAX_FIELD_LEN`] is almost
///    certainly accidental content passthrough (a tool name or file path
///    is never this long), so it is truncated with a `"...[redacted,
///    NNN bytes total]"` marker rather than emitted in full;
/// 2. control-character stripping -- embedded newlines (which could
///    forge additional fake log lines) are collapsed to spaces.
///
/// This is a defense-in-depth backstop, not the primary control: the
/// primary control is [`RequestRecord`]/[`FileSkipRecord`]'s own fixed,
/// exhaustive field lists, which structurally never carry request/
/// response payload text in the first place.
const MAX_FIELD_LEN: usize = 200;

/// The much tighter cap applied by [`redact_free_text`] -- see that
/// function's docs for why a free-text field needs a stricter bound than
/// [`redact`]'s general-purpose [`MAX_FIELD_LEN`].
const MAX_FREE_TEXT_LEN: usize = 40;

pub fn redact(value: &str) -> String {
    truncate_with_marker(value, MAX_FIELD_LEN)
}

/// Redact a FREE-TEXT diagnostic field (e.g. [`FileSkipRecord::reason`])
/// much more aggressively than [`redact`]'s general [`MAX_FIELD_LEN`].
///
/// [`redact`]'s length-based defense alone is insufficient for a
/// free-text field: a caller building a `reason` string by interpolating
/// real file content produces a value that, for a SMALL source file, may
/// never exceed [`MAX_FIELD_LEN`] at all -- the "no raw prompt/private
/// source text in diagnostics" guarantee cannot depend on the leaked
/// content happening to be long. `reason` is meant to be a short,
/// human-authored classification (`"no extractor for extension"`,
/// `"invalid UTF-8"`), never a copy of file contents, so capping it at
/// [`MAX_FREE_TEXT_LEN`] (40 chars -- comfortably longer than every
/// reason string this crate's own call sites use, per the redaction
/// test) is a correctness improvement, not a usability loss; a `path`
/// field, by contrast, legitimately needs [`MAX_FIELD_LEN`]'s wider
/// allowance and goes through plain [`redact`] instead.
pub fn redact_free_text(value: &str) -> String {
    truncate_with_marker(value, MAX_FREE_TEXT_LEN)
}

fn truncate_with_marker(value: &str, max_len: usize) -> String {
    let collapsed: String = value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if collapsed.len() <= max_len {
        collapsed
    } else {
        let total = collapsed.len();
        let mut cut = max_len;
        while cut > 0 && !collapsed.is_char_boundary(cut) {
            cut -= 1;
        }
        format!("{}...[redacted, {total} bytes total]", &collapsed[..cut])
    }
}

fn format_record(level: Level, record: &impl Record, format: Format) -> String {
    match format {
        Format::Text => format_text(level, record),
        Format::Json => format_json(level, record),
    }
}

fn format_text(level: Level, record: &impl Record) -> String {
    let mut parts = vec![
        format!("level={}", level.as_str()),
        format!("msg={}", record.event()),
    ];
    for (key, value) in record.fields() {
        parts.push(format!("{key}={}", quote_if_needed(&value)));
    }
    parts.join(" ")
}

fn quote_if_needed(value: &str) -> String {
    if value.chars().any(char::is_whitespace) || value.is_empty() {
        format!("{value:?}")
    } else {
        value.to_owned()
    }
}

fn format_json(level: Level, record: &impl Record) -> String {
    let mut map = serde_json::Map::new();
    map.insert("level".to_owned(), serde_json::json!(level.as_str()));
    map.insert("event".to_owned(), serde_json::json!(record.event()));
    for (key, value) in record.fields() {
        map.insert(key.to_owned(), serde_json::json!(value));
    }
    serde_json::Value::Object(map).to_string()
}

/// Emit `record` to real `stderr()`. The ONE call site in this crate that
/// touches a real stdio handle for diagnostics; carries the module's
/// narrow, documented allow (see module docs) rather than every caller
/// needing its own.
#[allow(clippy::print_stderr)]
pub fn emit_to_stderr(diagnostics: &Diagnostics, level: Level, record: &impl Record) {
    let mut stderr = std::io::stderr();
    // A diagnostics-emission failure (e.g. a closed stderr pipe) must
    // never crash the tool call it is describing -- best-effort only.
    let _ = diagnostics.emit(&mut stderr, level, record);
}
