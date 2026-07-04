//! d22 size/shape caps — the cross-stack `Validator` family mechanizing
//! ADBP_GAPS.md rows 91-94: a first-class size-cap family (file, function,
//! class/public-method-count, params, line-length, test-file) that every
//! ADBP agent stack and the CLAUDE.TEMPLATE mandate but that the legacy
//! `enforcer-rules` registry only covered generically (`SRC-2.1`), plus a
//! scored cyclomatic-complexity/nesting family, a scored package/path
//! nesting-depth family, and the per-file length metric the d02
//! baseline-grandfather-ratchet composes over.
//!
//! Every rule here is a lightweight line/keyword-oriented text detector
//! (mirroring [`crate::rules::fsm`]'s dominant shape) rather than a full
//! per-language AST parse — this crate has no tree-sitter/AST dependency
//! for Python/Dart/CFML/TS targets. T1 rules block (`Severity::Error`); T2
//! rules are SCORED — cyclomatic complexity/nesting accumulate a score
//! against a fixed threshold, and the ratchet metric emits `Warning`
//! (non-blocking at this per-file layer; the d02 baseline compares this
//! metric's raw signal against a recorded baseline to decide pass/fail at
//! the scan level).
//!
//! These rules EXTEND and complement the existing `SRC-*` shape rules —
//! this module does not edit them, and does not touch this crate's other
//! `rules::*` siblings ([`crate::rules::deferred_work`],
//! [`crate::rules::fsm`]).

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The fixed parts of one rule's finding: its id, severity, and title.
/// Bundled so the per-call-site `finding()` helper stays under clippy's
/// `too_many_arguments` limit while each validator keeps its own
/// rule-id/severity/title as a value at the call site (mirrors
/// [`crate::rules::fsm::FindingSpec`]).
struct FindingSpec<'a> {
    rule_id: &'a RuleId,
    severity: Severity,
    title: &'a str,
}

/// Build a [`Finding`] for one of this module's validators.
fn finding(
    spec: &FindingSpec<'_>,
    detail: String,
    input: &ValidationInput<'_>,
    line: u32,
) -> Finding {
    Finding {
        rule_id: spec.rule_id.clone(),
        severity: spec.severity,
        title: spec.title.to_owned(),
        detail,
        file: input.file.clone(),
        line,
        snippet: None,
    }
}

/// Total physical line count of `source`, counting a trailing partial
/// line (no final newline) as one more line, matching the intuitive
/// "how many lines does an editor show" count that the workpack's fixture
/// line counts (e.g. "201 lines") assume.
fn physical_line_count(source: &str) -> u32 {
    if source.is_empty() {
        return 0;
    }
    let newline_count = source.matches('\n').count() as u32;
    if source.ends_with('\n') {
        newline_count
    } else {
        newline_count.saturating_add(1)
    }
}

/// True when `path` is a Rust source file (`.rs`), for the per-language
/// file-length cap override.
fn is_rust_file(path: &str) -> bool {
    path.ends_with(".rs")
}

/// True when `path` names a test file (`.test.<ext>` or a `test_*`/
/// `*_test.<ext>` stem), for [`TestFileLengthValidator`]'s scope.
fn is_test_file(path: &str) -> bool {
    let file_name = path.rsplit('/').next().unwrap_or(path);
    file_name.contains(".test.") || file_name.starts_with("test_") || file_name.contains("_test.")
}

/// SIZE-FILE-1.1 — file length cap: a file over 200 physical lines is
/// flagged (T1), with a per-language override raising the cap to 400 for
/// Rust (`.rs`) files.
pub struct FileLengthValidator {
    rule_id: RuleId,
}

const FILE_LINE_CAP_DEFAULT: u32 = 200;
const FILE_LINE_CAP_RUST: u32 = 400;

impl FileLengthValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "SIZE-FILE-1.1".parse()?,
        })
    }
}

impl Validator for FileLengthValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        let cap = if is_rust_file(path) {
            FILE_LINE_CAP_RUST
        } else {
            FILE_LINE_CAP_DEFAULT
        };
        let total = physical_line_count(input.source);
        if total > cap {
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Error,
                    title: "size: file exceeds the line-count cap",
                },
                format!("`{path}` is {total} lines, over the {cap}-line file cap."),
                &input,
                1,
            )];
        }
        Vec::new()
    }
}

/// Markers that open a function-shaped block whose body this module scans
/// as one unit (mirrors the dominant `.mjs`-ported keyword-scan shape;
/// deliberately permissive across TS/JS/Python/Dart/Rust call sites).
const FUNCTION_OPENERS: &[&str] = &["function ", "fn ", "def "];

/// Find every top-level function block's line span by brace-matching from
/// an opener line to its balanced closing `}` (TS/JS/Dart/Rust shape) — or,
/// for a `def `-opened Python-shaped block with no braces, from the opener
/// to the next line at or below its own indent (a rough but adequate
/// text-level block-end heuristic for this crate's non-AST detectors).
fn function_blocks(source: &str) -> Vec<(u32, u32)> {
    let lines: Vec<&str> = source.lines().collect();
    let mut spans = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        let is_opener = FUNCTION_OPENERS.iter().any(|marker| line.contains(marker));
        if !is_opener {
            i += 1;
            continue;
        }
        let start = i;
        if line.contains('{') {
            let mut depth: i64 = 0;
            let mut j = i;
            let mut opened = false;
            while j < lines.len() {
                for ch in lines[j].chars() {
                    if ch == '{' {
                        depth += 1;
                        opened = true;
                    } else if ch == '}' {
                        depth -= 1;
                    }
                }
                if opened && depth <= 0 {
                    break;
                }
                j += 1;
            }
            spans.push((
                (start as u32).saturating_add(1),
                (j as u32).saturating_add(1),
            ));
            i = j + 1;
        } else {
            let indent = line.chars().take_while(|c| *c == ' ').count();
            let mut j = i + 1;
            while j < lines.len() {
                let next = lines[j];
                let next_indent = next.chars().take_while(|c| *c == ' ').count();
                if !next.trim().is_empty() && next_indent <= indent {
                    break;
                }
                j += 1;
            }
            spans.push(((start as u32).saturating_add(1), j as u32));
            i = j;
        }
    }
    spans
}

/// SIZE-FUNC-1.1 — function length cap: a function block over 30 physical
/// lines (signature through closing brace, inclusive) is flagged (T1).
pub struct FunctionLengthValidator {
    rule_id: RuleId,
}

const FUNCTION_LINE_CAP: u32 = 30;

impl FunctionLengthValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "SIZE-FUNC-1.1".parse()?,
        })
    }
}

impl Validator for FunctionLengthValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        for (start_line, end_line) in function_blocks(input.source) {
            let span = end_line.saturating_sub(start_line).saturating_add(1);
            if span > FUNCTION_LINE_CAP {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "size: function exceeds the line-count cap",
                    },
                    format!(
                        "This function spans {span} lines, over the {FUNCTION_LINE_CAP}-line \
                         function cap."
                    ),
                    &input,
                    start_line,
                )];
            }
        }
        Vec::new()
    }
}

/// Find every `class `-opened block's line span (brace-matched, TS/JS/Dart
/// shape) alongside its public-method count (lines inside the span
/// starting with `public ` or a bare method-shaped line with no
/// `private `/`protected `/`#`/leading-underscore marker, mirroring the
/// permissive default-is-public convention most of these languages share).
struct ClassBlock {
    start_line: u32,
    end_line: u32,
    public_method_count: u32,
}

fn class_blocks(source: &str) -> Vec<ClassBlock> {
    let lines: Vec<&str> = source.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        if !line.contains("class ") {
            i += 1;
            continue;
        }
        let start = i;
        let mut depth: i64 = 0;
        let mut j = i;
        let mut opened = false;
        while j < lines.len() {
            for ch in lines[j].chars() {
                if ch == '{' {
                    depth += 1;
                    opened = true;
                } else if ch == '}' {
                    depth -= 1;
                }
            }
            if opened && depth <= 0 {
                break;
            }
            j += 1;
        }
        let mut public_method_count = 0u32;
        for body_line in lines.iter().take(j + 1).skip(start + 1) {
            let trimmed = body_line.trim_start();
            let is_method_signature = trimmed.contains('(') && trimmed.contains(')');
            let is_private_or_protected = trimmed.starts_with("private ")
                || trimmed.starts_with("protected ")
                || trimmed.starts_with('_')
                || trimmed.starts_with('#');
            if is_method_signature && !is_private_or_protected {
                public_method_count += 1;
            }
        }
        blocks.push(ClassBlock {
            start_line: (start as u32).saturating_add(1),
            end_line: (j as u32).saturating_add(1),
            public_method_count,
        });
        i = j + 1;
    }
    blocks
}

/// SIZE-CLASS-1.1 — class length + public-method-count cap: a class block
/// over 150 physical lines, OR carrying more than 12 public methods, is
/// flagged (T1).
pub struct ClassSizeValidator {
    rule_id: RuleId,
}

const CLASS_LINE_CAP: u32 = 150;
const CLASS_PUBLIC_METHOD_CAP: u32 = 12;

impl ClassSizeValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "SIZE-CLASS-1.1".parse()?,
        })
    }
}

impl Validator for ClassSizeValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        for block in class_blocks(input.source) {
            let span = block
                .end_line
                .saturating_sub(block.start_line)
                .saturating_add(1);
            if span > CLASS_LINE_CAP {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "size: class exceeds the line-count cap",
                    },
                    format!(
                        "This class spans {span} lines, over the {CLASS_LINE_CAP}-line class cap."
                    ),
                    &input,
                    block.start_line,
                )];
            }
            if block.public_method_count > CLASS_PUBLIC_METHOD_CAP {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "size: class exceeds the public-method-count cap",
                    },
                    format!(
                        "This class declares {} public methods, over the \
                         {CLASS_PUBLIC_METHOD_CAP}-method cap.",
                        block.public_method_count
                    ),
                    &input,
                    block.start_line,
                )];
            }
        }
        Vec::new()
    }
}

/// Count the comma-separated parameters in a function/method signature's
/// parenthesized parameter list on `line`. Returns `None` when the line
/// has no non-empty parenthesized list (e.g. a zero-arg signature, or a
/// line this heuristic does not recognize as a signature at all).
fn param_count_in_signature(line: &str) -> Option<u32> {
    let open = line.find('(')?;
    let close = line[open..].find(')').map(|i| open + i)?;
    let inner = line[open + 1..close].trim();
    if inner.is_empty() {
        return None;
    }
    Some((inner.matches(',').count() as u32).saturating_add(1))
}

/// SIZE-PARAMS-1.1 — parameter-count cap: a function/method signature with
/// more than 5 parameters is flagged (T1).
pub struct ParamCountValidator {
    rule_id: RuleId,
}

const PARAM_COUNT_CAP: u32 = 5;

impl ParamCountValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "SIZE-PARAMS-1.1".parse()?,
        })
    }
}

impl Validator for ParamCountValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        for (line_idx, line) in input.source.lines().enumerate() {
            let is_signature_line = FUNCTION_OPENERS.iter().any(|marker| line.contains(marker));
            if !is_signature_line {
                continue;
            }
            if let Some(count) = param_count_in_signature(line) {
                if count > PARAM_COUNT_CAP {
                    return vec![finding(
                        &FindingSpec {
                            rule_id: &self.rule_id,
                            severity: Severity::Error,
                            title: "size: signature exceeds the parameter-count cap",
                        },
                        format!(
                            "This signature declares {count} parameters, over the \
                             {PARAM_COUNT_CAP}-parameter cap."
                        ),
                        &input,
                        (line_idx as u32).saturating_add(1),
                    )];
                }
            }
        }
        Vec::new()
    }
}

/// SIZE-LINE-1.1 — line-length cap: a raw physical line over 120 columns
/// is flagged (T1), measured over the FULL line INCLUDING any trailing
/// pragma/comment — a trailing suppression tail does not exempt the line.
pub struct LineLengthValidator {
    rule_id: RuleId,
}

const LINE_LENGTH_CAP: usize = 120;

impl LineLengthValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "SIZE-LINE-1.1".parse()?,
        })
    }
}

impl Validator for LineLengthValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        for (line_idx, line) in input.source.lines().enumerate() {
            let len = line.chars().count();
            if len > LINE_LENGTH_CAP {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "size: line exceeds the column-length cap",
                    },
                    format!(
                        "This line is {len} columns (including any trailing pragma/comment), \
                         over the {LINE_LENGTH_CAP}-column cap."
                    ),
                    &input,
                    (line_idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// SIZE-TESTFILE-1.1 — test-file length cap: a test file (`.test.<ext>` /
/// `test_*`/`*_test.<ext>`) over 300 physical lines is flagged (T1). Scope
/// is a length metric only; test *content* quality is d23's rule.
pub struct TestFileLengthValidator {
    rule_id: RuleId,
}

const TEST_FILE_LINE_CAP: u32 = 300;

impl TestFileLengthValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "SIZE-TESTFILE-1.1".parse()?,
        })
    }
}

impl Validator for TestFileLengthValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        if !is_test_file(path) {
            return Vec::new();
        }
        let total = physical_line_count(input.source);
        if total > TEST_FILE_LINE_CAP {
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Error,
                    title: "size: test file exceeds the line-count cap",
                },
                format!(
                    "`{path}` is {total} lines, over the {TEST_FILE_LINE_CAP}-line test-file cap."
                ),
                &input,
                1,
            )];
        }
        Vec::new()
    }
}

/// Decision-point keyword markers this module's scored cyclomatic-
/// complexity model counts, one point per occurrence (a text-level stand-in
/// for the AST branch count an eventual `syn`/`tree-sitter` pass would
/// compute exactly).
const DECISION_POINT_MARKERS: &[&str] = &[
    "if (", "if(", "for (", "for(", "while (", "while(", "catch (", "catch(", "case ",
];

/// Score threshold at/above which [`ComplexityNestingValidator`] fires:
/// cyclomatic complexity (decision points + 1) `>= 10`, OR max nesting
/// depth `> 3`.
const COMPLEXITY_FIRE_THRESHOLD: u32 = 10;
const NESTING_DEPTH_CAP: u32 = 3;

/// Count this file's decision points (one per [`DECISION_POINT_MARKERS`]
/// occurrence) and its deepest brace-nesting level, in one pass.
fn complexity_and_nesting(source: &str) -> (u32, u32) {
    let mut decision_points = 0u32;
    for line in source.lines() {
        for marker in DECISION_POINT_MARKERS {
            decision_points = decision_points.saturating_add(line.matches(marker).count() as u32);
        }
    }
    let mut depth: i64 = 0;
    let mut max_depth: i64 = 0;
    for ch in source.chars() {
        match ch {
            '{' => {
                depth += 1;
                max_depth = max_depth.max(depth);
            }
            '}' => depth -= 1,
            _ => {}
        }
    }
    // Nesting depth relative to the enclosing function body (depth 1 is
    // the function's own top-level body, not "nested" yet).
    let nesting_depth = max_depth.saturating_sub(1).max(0) as u32;
    (decision_points, nesting_depth)
}

/// SIZE-CX-1.1 — scored cyclomatic complexity / nesting depth: a function
/// scoring cyclomatic complexity >= 10 (decision points + 1) OR nesting
/// depth > 3 is flagged (T2, non-blocking, scored against a fixed
/// threshold per this crate's established scored-family convention — see
/// [`crate::rules::fsm::FsmTransitionCoverageValidator`]).
pub struct ComplexityNestingValidator {
    rule_id: RuleId,
}

impl ComplexityNestingValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "SIZE-CX-1.1".parse()?,
        })
    }
}

impl Validator for ComplexityNestingValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let (decision_points, nesting_depth) = complexity_and_nesting(input.source);
        let cyclomatic_complexity = decision_points.saturating_add(1);
        if cyclomatic_complexity >= COMPLEXITY_FIRE_THRESHOLD || nesting_depth > NESTING_DEPTH_CAP {
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Warning,
                    title: "size: cyclomatic complexity or nesting depth crosses threshold",
                },
                format!(
                    "Scored cyclomatic complexity {cyclomatic_complexity} (threshold \
                     {COMPLEXITY_FIRE_THRESHOLD}) / nesting depth {nesting_depth} (cap \
                     {NESTING_DEPTH_CAP})."
                ),
                &input,
                1,
            )];
        }
        Vec::new()
    }
}

/// Path segment this validator treats as the package root anchor: nesting
/// depth is measured in directory segments AFTER this anchor, matching the
/// legacy `py-fastapi-package-nesting-depth` rule's own `app/` convention.
const PACKAGE_ROOT_ANCHOR: &str = "app/";

/// SIZE-NEST-1.1 (legacy `py-fastapi-package-nesting-depth`) — scored
/// package/path nesting-depth: a file whose directory path nests more
/// than 3 levels deep under the package root is flagged (T2, scored).
pub struct PackageNestingDepthValidator {
    rule_id: RuleId,
}

const PACKAGE_NESTING_DEPTH_CAP: usize = 3;

impl PackageNestingDepthValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "SIZE-NEST-1.1".parse()?,
        })
    }
}

impl Validator for PackageNestingDepthValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        let Some(after_anchor) = path.split_once(PACKAGE_ROOT_ANCHOR).map(|(_, rest)| rest) else {
            return Vec::new();
        };
        let depth = after_anchor.matches('/').count();
        if depth > PACKAGE_NESTING_DEPTH_CAP {
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Warning,
                    title: "size: package/path nesting depth crosses threshold",
                },
                format!(
                    "`{path}` nests {depth} directory levels under `{PACKAGE_ROOT_ANCHOR}`, over \
                     the {PACKAGE_NESTING_DEPTH_CAP}-level cap."
                ),
                &input,
                1,
            )];
        }
        Vec::new()
    }
}

/// Prefix of the recorded-baseline marker comment [`RatchetLengthValidator`]
/// reads to know a file's LAST accepted length, e.g. `// enforcer-baseline:
/// 205`. Composes with d02: this validator supplies the per-file length
/// metric only; the recorded baseline STORE and the ratchet-down-on-fix
/// mechanics live in `enforcer-scan`'s `rules::baseline_ratchet` (d02),
/// which this crate does not depend on (wrong dependency direction —
/// `enforcer-scan` depends on this crate, not the reverse).
const BASELINE_MARKER_PREFIX: &str = "enforcer-baseline:";

/// Parse the recorded baseline line-count out of a `// enforcer-baseline:
/// N` marker comment, if present anywhere in the file.
fn recorded_baseline(source: &str) -> Option<u32> {
    for line in source.lines() {
        if let Some(idx) = line.find(BASELINE_MARKER_PREFIX) {
            let rest = line[idx + BASELINE_MARKER_PREFIX.len()..].trim();
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(value) = digits.parse::<u32>() {
                return Some(value);
            }
        }
    }
    None
}

/// FE-LEN-2.1 — grandfather-ratchet length metric: a file carrying a
/// recorded-baseline marker is clean while at or below its recorded
/// baseline, and flagged (T2, non-blocking at this per-file layer) once it
/// grows past that baseline. This is the per-file METRIC only; the
/// baseline STORE and true "fail the run" ratchet decision compose with
/// d02's `enforcer-scan::rules::baseline_ratchet`.
pub struct RatchetLengthValidator {
    rule_id: RuleId,
}

impl RatchetLengthValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "FE-LEN-2.1".parse()?,
        })
    }
}

impl Validator for RatchetLengthValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(baseline) = recorded_baseline(input.source) else {
            return Vec::new();
        };
        let total = physical_line_count(input.source);
        if total > baseline {
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Warning,
                    title: "size: file grew past its recorded baseline",
                },
                format!(
                    "`{}` is {total} lines, over its recorded baseline of {baseline} lines \
                     (grandfather ratchet: warn-at-baseline, fail-if-grown).",
                    input.file.as_str()
                ),
                &input,
                1,
            )];
        }
        Vec::new()
    }
}

/// Build every `size_shape` family validator this crate owns (d22).
pub fn validators() -> Result<Vec<Box<dyn Validator>>, enforcer_core::error::DecodeError> {
    Ok(vec![
        Box::new(FileLengthValidator::new()?),
        Box::new(FunctionLengthValidator::new()?),
        Box::new(ClassSizeValidator::new()?),
        Box::new(ParamCountValidator::new()?),
        Box::new(LineLengthValidator::new()?),
        Box::new(TestFileLengthValidator::new()?),
        Box::new(ComplexityNestingValidator::new()?),
        Box::new(PackageNestingDepthValidator::new()?),
        Box::new(RatchetLengthValidator::new()?),
    ])
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::*;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn nine_validators_registered_with_unique_rule_ids(
    ) -> Result<(), enforcer_core::error::DecodeError> {
        let vs = validators()?;
        assert_eq!(vs.len(), 9);
        let mut seen = std::collections::BTreeSet::new();
        for v in &vs {
            assert!(seen.insert(v.rule_id().to_string()));
        }
        assert_eq!(seen.len(), 9);
        Ok(())
    }

    #[test]
    fn size_file_length() -> Result<(), Box<dyn std::error::Error>> {
        let validator = FileLengthValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/size_shape/file/bad.ts",
            "tests/fixtures/size_shape/file/good.ts",
        )?;
        Ok(())
    }

    #[test]
    fn size_file_length_rust_override() -> Result<(), Box<dyn std::error::Error>> {
        let validator = FileLengthValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/size_shape/file_rust/bad.rs",
            "tests/fixtures/size_shape/file_rust/good.rs",
        )?;
        Ok(())
    }

    #[test]
    fn size_function_length() -> Result<(), Box<dyn std::error::Error>> {
        let validator = FunctionLengthValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/size_shape/func/bad.ts",
            "tests/fixtures/size_shape/func/good.ts",
        )?;
        Ok(())
    }

    #[test]
    fn size_class_length() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ClassSizeValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/size_shape/class/bad.ts",
            "tests/fixtures/size_shape/class/good.ts",
        )?;
        Ok(())
    }

    #[test]
    fn size_class_public_method_count() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ClassSizeValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/size_shape/class_methods/bad.ts",
            "tests/fixtures/size_shape/class_methods/good.ts",
        )?;
        Ok(())
    }

    #[test]
    fn size_param_count() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ParamCountValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/size_shape/params/bad.ts",
            "tests/fixtures/size_shape/params/good.ts",
        )?;
        Ok(())
    }

    #[test]
    fn size_line_length() -> Result<(), Box<dyn std::error::Error>> {
        let validator = LineLengthValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/size_shape/line/bad.ts",
            "tests/fixtures/size_shape/line/good.ts",
        )?;
        Ok(())
    }

    #[test]
    fn size_line_length_trailing_pragma_still_flags() -> Result<(), Box<dyn std::error::Error>> {
        let validator = LineLengthValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/size_shape/line_pragma/bad.ts",
            "tests/fixtures/size_shape/line/good.ts",
        )?;
        Ok(())
    }

    #[test]
    fn size_testfile_length() -> Result<(), Box<dyn std::error::Error>> {
        let validator = TestFileLengthValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/size_shape/testfile/bad.test.ts",
            "tests/fixtures/size_shape/testfile/good.test.ts",
        )?;
        Ok(())
    }

    #[test]
    fn size_complexity_nesting() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ComplexityNestingValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/size_shape/cx/bad.ts",
            "tests/fixtures/size_shape/cx/good.ts",
        )?;
        Ok(())
    }

    #[test]
    fn size_package_nesting_depth() -> Result<(), Box<dyn std::error::Error>> {
        let validator = PackageNestingDepthValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/size_shape/nesting/bad/app/core/services/domain/user/service.py",
            "tests/fixtures/size_shape/nesting/good/app/user_service.py",
        )?;
        Ok(())
    }

    #[test]
    fn size_ratchet_length() -> Result<(), Box<dyn std::error::Error>> {
        let validator = RatchetLengthValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/size_shape/ratchet/bad/legacy.ts",
            "tests/fixtures/size_shape/ratchet/good/legacy.ts",
        )?;
        Ok(())
    }
}
