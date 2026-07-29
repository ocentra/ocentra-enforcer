//! X06.7: structured diagnostics for the MCP/CLI/watch surface.
//!
//! # stdout purity (hard requirement)
//!
//! Per the baseline schema doc Â§1 ("structured KV logs to stderr ONLY --
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
//! `refs/x06-baseline-tool-schemas.md` Â§1)
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

use crate::owned_boundary::{Retained, RetainedDisplay};
use enforcer_domain::memory_types::{
    CodeSearchQuantity, MemoryDiagnosticFieldValue, MemoryDiagnosticFilePath,
    MemoryDiagnosticFreeText, MemoryDiagnosticIsError, MemoryDiagnosticMethod,
    MemoryDiagnosticProtocol, MemoryDiagnosticRedactedValue, MemoryDiagnosticRequestDuration,
    MemoryDiagnosticSkipReason, MemoryDiagnosticTool, ParserSourceText,
};
use enforcer_domain::memory_types::{Format, Level, SkipPhase};
use std::io::Write;

/// Log severity. Ordinal order matches the baseline's `CBM_LOG_*`
/// constants exactly (`debug=0, info=1, warn=2, error=3, none=4`, binding:
/// `refs/x06-baseline-tool-schemas.md` Â§1) so the numeric env-var forms
/// this crate accepts (see [`Level::from_env_str`]) resolve to the same
/// level a baseline-familiar operator would expect from that number.
/// [`Level::None`] is a configured-minimum-only sentinel (never a record's
/// own severity) that disables all logging, matching `CBM_LOG_NONE`.
/// Output encoding for emitted diagnostic lines.
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
        let level = match std::env::var("ENFORCER_MEMORY_LOG_LEVEL") {
            Ok(value) => Level::from_env_str(&value).unwrap_or(Level::Info),
            Err(_) => Level::Info,
        };
        let format = match std::env::var("ENFORCER_MEMORY_LOG_FORMAT") {
            Ok(value) => match value.to_ascii_lowercase().as_str() {
                "json" => Some(Format::Json),
                "text" => Some(Format::Text),
                _ => None,
            }
            .unwrap_or(Format::Text),
            Err(_) => Format::Text,
        };
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
        writeln!(sink, "{}", line.as_str())
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
    /// `refs/x06-baseline-tool-schemas.md` Â§1).
    fn event(&self) -> MemoryDiagnosticFreeText;
    /// Ordered `(key, value)` pairs, values ALREADY passed through
    /// [`redact`] by the implementor -- this trait does not re-redact,
    /// so every implementor is individually responsible (both this
    /// module's two record types satisfy it; the redaction test covers
    /// both).
    fn fields(&self) -> Vec<(MemoryDiagnosticFreeText, MemoryDiagnosticRedactedValue)>;
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
    pub protocol: MemoryDiagnosticProtocol,
    /// The JSON-RPC method (`"tools/call"`, `"initialize"`, ...) or, for
    /// the CLI transport, the CLI's own invocation shape (`"cli"`).
    pub method: MemoryDiagnosticMethod,
    /// The tool name, when this request was a `tools/call`/CLI tool
    /// invocation; `None` for `initialize`/`ping`/`tools/list`.
    pub tool: Option<MemoryDiagnosticTool>,
    pub duration: MemoryDiagnosticRequestDuration,
    pub is_error: MemoryDiagnosticIsError,
}

impl RequestRecord {
    /// The level this record should be emitted at: WARN on error, INFO
    /// otherwise (binding spec).
    pub fn level(&self) -> Level {
        if self.is_error.is_error() {
            Level::Warn
        } else {
            Level::Info
        }
    }
}

impl Record for RequestRecord {
    fn event(&self) -> MemoryDiagnosticFreeText {
        "mcp.request".into()
    }

    fn fields(&self) -> Vec<(MemoryDiagnosticFreeText, MemoryDiagnosticRedactedValue)> {
        let status = if self.is_error.is_error() {
            "error"
        } else {
            "ok"
        };
        vec![
            ("protocol".into(), self.protocol.retained_display().into()),
            (
                "method".into(),
                redact(&MemoryDiagnosticFieldValue::from(self.method.as_str())),
            ),
            (
                "tool".into(),
                self.tool
                    .as_deref()
                    .map(|tool| redact(&MemoryDiagnosticFieldValue::from(tool)))
                    .unwrap_or_else(|| "".into()),
            ),
            ("status".into(), status.retained().into()),
            (
                "durationMs".into(),
                self.duration.get().as_millis().retained_display().into(),
            ),
        ]
    }
}

/// Which indexing phase a [`FileSkipRecord`] occurred in.
/// One per-file skip diagnostic: path, human reason, and which phase
/// skipped it. Per the workpack's "never silent skip" doctrine, this is
/// the record type callers emit instead of dropping a file with no
/// trace. The `path` is the repo-relative path only (never file
/// CONTENTS), and [`redact`] still runs over both string fields as a
/// defense-in-depth measure against a `reason` string that was built by
/// interpolating file content by mistake.
#[derive(Debug, Clone, PartialEq)]
pub struct FileSkipRecord {
    pub path: MemoryDiagnosticFilePath,
    pub reason: MemoryDiagnosticSkipReason,
    pub phase: SkipPhase,
}

impl Record for FileSkipRecord {
    fn event(&self) -> MemoryDiagnosticFreeText {
        "file_skip".into()
    }

    fn fields(&self) -> Vec<(MemoryDiagnosticFreeText, MemoryDiagnosticRedactedValue)> {
        vec![
            (
                "path".into(),
                redact(&MemoryDiagnosticFieldValue::from(self.path.as_str())),
            ),
            (
                "reason".into(),
                redact_free_text(&MemoryDiagnosticFreeText::from(self.reason.as_str())),
            ),
            ("phase".into(), self.phase.retained_display().into()),
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
///    forge additional synthetic log lines) are collapsed to spaces.
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

pub fn redact(value: &MemoryDiagnosticFieldValue) -> MemoryDiagnosticRedactedValue {
    truncate_with_marker(value.as_str().into(), MAX_FIELD_LEN.into())
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
pub fn redact_free_text(value: &MemoryDiagnosticFreeText) -> MemoryDiagnosticRedactedValue {
    truncate_with_marker(value.as_str().into(), MAX_FREE_TEXT_LEN.into())
}

fn truncate_with_marker(
    value: ParserSourceText<'_>,
    max_len: CodeSearchQuantity,
) -> MemoryDiagnosticRedactedValue {
    let collapsed: String = value
        .as_str()
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if collapsed.len() <= max_len.get() {
        collapsed.into()
    } else {
        let total = collapsed.len();
        let mut cut = max_len.get();
        while cut > 0 && !collapsed.is_char_boundary(cut) {
            cut -= 1;
        }
        format!(
            "{}...[redacted, {total} bytes total]",
            collapsed.get(..cut).map_or("", |prefix| prefix)
        )
        .into()
    }
}

fn format_record(
    level: Level,
    record: &impl Record,
    format: Format,
) -> MemoryDiagnosticRedactedValue {
    match format {
        Format::Text => format_text(level, record),
        Format::Json => format_json(level, record),
    }
}

fn format_text(level: Level, record: &impl Record) -> MemoryDiagnosticRedactedValue {
    let mut parts = vec![
        format!("level={}", level.as_str()),
        format!("msg={}", record.event().as_str()),
    ];
    for (key, value) in record.fields() {
        parts.push(format!(
            "{}={}",
            key.as_str(),
            quote_if_needed(value.as_str().into()).as_str()
        ));
    }
    parts.join(" ").into()
}

fn quote_if_needed(value: ParserSourceText<'_>) -> MemoryDiagnosticRedactedValue {
    if value.as_str().chars().any(char::is_whitespace) || value.as_str().is_empty() {
        format!("{:?}", value.as_str()).into()
    } else {
        value.as_str().retained().into()
    }
}

fn format_json(level: Level, record: &impl Record) -> MemoryDiagnosticRedactedValue {
    let mut map = std::collections::BTreeMap::new();
    // ALLOC-JUSTIFICATION: JSON serialization owns map keys and values so the
    // diagnostic record can be formatted after the borrowed Record is gone.
    map.insert("level".retained(), level.as_str().to_owned());
    map.insert("event".retained(), record.event().as_str().to_owned());
    for (key, value) in record.fields() {
        // ALLOC-JUSTIFICATION: each dynamic field must be owned by the JSON
        // map because the source record remains borrowed only for this call.
        map.insert(key.as_str().to_owned(), value.as_str().to_owned());
    }
    // ALLOC-JUSTIFICATION: the fallback is an owned JSON payload required when
    // serialization fails, preserving the formatter's non-panicking contract.
    serde_json::to_string(&map)
        .unwrap_or_else(|_| "{}".to_owned())
        .into()
}

/// Emit `record` to real `stderr()`. The ONE call site in this crate that
/// touches a real stdio handle for diagnostics; carries the module's
/// narrow, documented allow (see module docs) rather than every caller
/// needing its own.
pub fn emit_to_stderr(diagnostics: &Diagnostics, level: Level, record: &impl Record) {
    let mut stderr = std::io::stderr();
    // A diagnostics-emission failure (e.g. a closed stderr pipe) must
    // never crash the tool call it is describing -- best-effort only.
    let _ = diagnostics.emit(&mut stderr, level, record);
}
