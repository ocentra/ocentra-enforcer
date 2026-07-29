//! Raw source-text analysis owned by the common-language input boundary.
//!
//! BOUNDARY-INVARIANT: untrusted source text is reduced to bounded, typed
//! marker observations before any common validator consumes it.
//! boundaryOwnerNote: enforcer-lang-common owns source marker classification;
//! edits to this boundary stay covered by the crate's parity and fixture tests.

use crate::error::DeferredAnnotationError;
use enforcer_domain::config_types::Platform;
use enforcer_domain::paths::RelPath;

#[derive(Debug)]
pub(crate) struct PatternMarkers(Vec<&'static str>);

impl PatternMarkers {
    pub(crate) fn new(values: impl Into<Vec<&'static str>>) -> Self {
        Self(values.into())
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &&'static str> {
        self.0.iter()
    }
}

#[derive(Debug)]
pub(crate) struct PlatformMarker {
    pub(crate) platform: Platform,
    pub(crate) marker: &'static str,
}

impl PlatformMarker {
    pub(crate) fn matches(&self, line: &str) -> bool {
        line.match_indices(self.marker).any(|(start, _)| {
            let before = line
                .get(..start)
                .and_then(|prefix| prefix.chars().next_back());
            let after = start
                .checked_add(self.marker.len())
                .and_then(|end| line.get(end..))
                .and_then(|suffix| suffix.chars().next());

            if self.marker.starts_with('.') {
                !after.is_some_and(is_identifier_continuation)
            } else {
                !before.is_some_and(is_identifier_continuation)
                    && !after.is_some_and(is_identifier_continuation)
            }
        })
    }
}

fn is_identifier_continuation(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

pub(crate) const PLATFORM_MARKERS: &[PlatformMarker] = &[
    PlatformMarker {
        platform: Platform::Windows,
        marker: ".ps1",
    },
    PlatformMarker {
        platform: Platform::Windows,
        marker: ".cmd",
    },
    PlatformMarker {
        platform: Platform::Windows,
        marker: ".bat",
    },
    PlatformMarker {
        platform: Platform::Macos,
        marker: "osascript",
    },
    PlatformMarker {
        platform: Platform::Linux,
        marker: ".sh",
    },
];

pub(crate) const GUARD_MARKERS: &[&str] =
    &["process.platform", "uname", "$OSTYPE", "cfg(target_os"];

pub(crate) fn platform_label(platform: Platform) -> &'static str {
    match platform {
        Platform::Windows => "windows",
        Platform::Macos => "macos",
        Platform::Linux => "linux",
    }
}

pub(crate) fn first_line_containing(source: &str, marker: &str) -> Option<u32> {
    source
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains(marker))
        .map(|(index, _)| crate::boundary::line_number(index))
}

pub(crate) fn physical_line_count(source: &str) -> u32 {
    if source.is_empty() {
        return 0;
    }
    let newline_count = crate::boundary::count(source.matches('\n').count());
    if source.ends_with('\n') {
        newline_count
    } else {
        newline_count.saturating_add(1)
    }
}

pub(crate) fn is_rust_file(path: &RelPath) -> bool {
    path.as_str().ends_with(".rs")
}

pub(crate) fn is_test_file(path: &RelPath) -> bool {
    let file_name = path.as_str().rsplit('/').next().unwrap_or(path.as_str());
    file_name.contains(".test.") || file_name.starts_with("test_") || file_name.contains("_test.")
}

/// Return whether a path is in the script surface governed by `PORT-1.1`.
///
/// Portability is a script-boundary rule, not a rule over every source file
/// that happens to mention a shell extension. Keeping the path decision here
/// prevents the validator's own marker tables, prose, and test source from
/// becoming false positives while preserving all commands in `scripts/**`.
/// The port-1 fixture directory is included so fixture parity exercises the
/// exact same production path contract.
pub(crate) fn is_portability_target(path: &RelPath) -> bool {
    let normalized = path.as_str().replace('\\', "/");
    let lower = normalized.to_ascii_lowercase();
    if lower.contains("/generated/")
        || lower.contains("/templates/")
        || lower.ends_with(".template")
        || lower.contains(".template.")
    {
        return false;
    }
    lower == "scripts"
        || lower.starts_with("scripts/")
        || lower.contains("/scripts/")
        || lower.starts_with("fixtures/port-1/")
}

/// Documentation-only lines do not execute a platform-specific command.
/// Ignore the comment forms shared by the script and source languages this
/// crate scans, while leaving an executable line with a trailing comment
/// governed by `PORT-1.1`.
pub(crate) fn is_documentation_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//")
        || trimmed.starts_with("#")
        || trimmed.starts_with("/*")
        || trimmed.starts_with("*")
        || trimmed.starts_with("<!--")
}

/// Parses raw boundary declarations before the validator applies its DTO budget.
/// This is deliberately boundary-owned because it handles untrusted source text.
pub(crate) fn has_fallible_domain_conversion(source: &str, name: &str) -> bool {
    let conversion = format!("impl TryFrom<{name}>");
    source
        .lines()
        .map(str::trim_start)
        .any(|line| line.starts_with(&conversion))
}

/// Extracts a DTO-like declaration name from one raw boundary-source line.
pub(crate) fn boundary_declaration_name(line: &str) -> Option<&str> {
    let text = line.trim_start();
    let declaration = ["pub struct ", "struct ", "pub enum ", "enum "]
        .into_iter()
        .find_map(|prefix| text.strip_prefix(prefix))?;
    let name = declaration
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .next()?;
    (name.contains("Dto")
        || name.contains("DTO")
        || name.contains("Payload")
        || name.contains("Request"))
    .then_some(name)
}

/// Counts raw public fields in DTO-like declarations before the budget rule
/// converts the observation into a typed finding.
pub(crate) fn raw_public_boundary_declarations(source: &str) -> usize {
    let mut count = 0;
    let mut declaration_name: Option<String> = None;
    let mut has_raw_field = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(name) = boundary_declaration_name(trimmed) {
            declaration_name = Some(name.into());
            has_raw_field = false;
            let Some((_, fields)) = trimmed.split_once('{') else {
                continue;
            };
            has_raw_field = raw_public_boundary_field(fields);
            if fields.contains('}') {
                if let Some(name) = declaration_name.take() {
                    count += usize::from(
                        has_raw_field && !has_fallible_domain_conversion(source, &name),
                    );
                }
            }
            continue;
        }
        if declaration_name.is_none() {
            continue;
        }
        has_raw_field |= raw_public_boundary_field(trimmed);
        if trimmed.starts_with('}') {
            if let Some(name) = declaration_name.take() {
                count +=
                    usize::from(has_raw_field && !has_fallible_domain_conversion(source, &name));
            }
        }
    }
    count
}

fn raw_public_boundary_field(fields: &str) -> bool {
    fields.contains("pub ")
        && [
            "String",
            "str",
            "u8",
            "u16",
            "u32",
            "u64",
            "usize",
            "i8",
            "i16",
            "i32",
            "i64",
            "isize",
            "f32",
            "f64",
            "bool",
            "serde_json::Value",
        ]
        .iter()
        .any(|raw| {
            fields.contains(&format!(": {raw}")) || fields.contains(&format!(": Option<{raw}"))
        })
}

const FUNCTION_OPENERS: &[&str] = &["function ", "fn ", "def "];

pub(crate) fn is_function_opener(line: &str) -> bool {
    FUNCTION_OPENERS.iter().any(|marker| line.contains(marker))
}

#[derive(Debug)]
pub(crate) struct FunctionBlock {
    pub(crate) start_line: u32,
    pub(crate) end_line: u32,
}

pub(crate) fn function_blocks(source: &str) -> Vec<FunctionBlock> {
    let lines: Vec<&str> = source.lines().collect();
    let mut spans = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let Some(&line) = lines.get(i) else {
            break;
        };
        if !is_function_opener(line) {
            i += 1;
            continue;
        }
        let start = i;
        if line.contains('{') {
            let mut depth: i64 = 0;
            let mut j = i;
            let mut opened = false;
            while j < lines.len() {
                let Some(&current) = lines.get(j) else {
                    break;
                };
                for ch in current.chars() {
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
            spans.push(FunctionBlock {
                start_line: crate::boundary::line_number(start),
                end_line: crate::boundary::line_number(j),
            });
            i = j + 1;
        } else {
            let indent = line.chars().take_while(|c| *c == ' ').count();
            let mut j = i + 1;
            while j < lines.len() {
                let Some(&next) = lines.get(j) else {
                    break;
                };
                let next_indent = next.chars().take_while(|c| *c == ' ').count();
                if !next.trim().is_empty() && next_indent <= indent {
                    break;
                }
                j += 1;
            }
            spans.push(FunctionBlock {
                start_line: crate::boundary::line_number(start),
                end_line: crate::boundary::count(j),
            });
            i = j;
        }
    }
    spans
}

#[derive(Debug)]
pub(crate) struct ClassBlock {
    pub(crate) start_line: u32,
    pub(crate) end_line: u32,
    pub(crate) public_method_count: u32,
}

pub(crate) fn class_blocks(source: &str) -> Vec<ClassBlock> {
    let lines: Vec<&str> = source.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let Some(&line) = lines.get(i) else {
            break;
        };
        if !line.contains("class ") {
            i += 1;
            continue;
        }
        let start = i;
        let mut depth: i64 = 0;
        let mut j = i;
        let mut opened = false;
        while j < lines.len() {
            let Some(&current) = lines.get(j) else {
                break;
            };
            for ch in current.chars() {
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
            start_line: crate::boundary::line_number(start),
            end_line: crate::boundary::line_number(j),
            public_method_count,
        });
        i = j + 1;
    }
    blocks
}

pub(crate) fn param_count_in_signature(line: &str) -> Option<u32> {
    let open = line.find('(')?;
    let after_open = line.get(open..)?;
    let close = after_open
        .find(')')
        .and_then(|offset| open.checked_add(offset))?;
    let inner = line.get(open.checked_add(1)?..close)?.trim();
    (!inner.is_empty()).then(|| crate::boundary::line_number(inner.matches(',').count()))
}

#[derive(Debug)]
pub(crate) struct ComplexityMetrics {
    pub(crate) decision_points: u32,
    pub(crate) nesting_depth: u32,
}

pub(crate) fn complexity_and_nesting(source: &str) -> ComplexityMetrics {
    const DECISION_POINT_MARKERS: &[&str] = &[
        "if (", "if(", "for (", "for(", "while (", "while(", "catch (", "catch(", "case ",
    ];
    let mut decision_points = 0u32;
    for line in source.lines() {
        for marker in DECISION_POINT_MARKERS {
            decision_points = decision_points
                .saturating_add(crate::boundary::count(line.matches(marker).count()));
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
    ComplexityMetrics {
        decision_points,
        nesting_depth: crate::boundary::nonnegative_count(max_depth.saturating_sub(1)),
    }
}

pub(crate) fn recorded_baseline(source: &str) -> Option<u32> {
    for line in source.lines() {
        if let Some((_, rest)) = line.split_once("enforcer-baseline:") {
            let digits: String = rest
                .trim()
                .chars()
                .take_while(|character| character.is_ascii_digit())
                .collect();
            if let Ok(value) = digits.parse::<u32>() {
                return Some(value);
            }
        }
    }
    None
}

pub(crate) fn test_names(source: &str) -> Vec<String> {
    const TEST_CASE_OPENERS: &[&str] = &["it(", "test(", "it.only(", "test.only("];
    let mut names = Vec::new();
    for line in source.lines() {
        for opener in TEST_CASE_OPENERS {
            if let Some(rest) = line.trim_start().strip_prefix(opener) {
                if let Some(quote @ ('"' | '\'')) = rest.chars().next() {
                    if let Some((name, _)) =
                        rest.strip_prefix(quote).and_then(|v| v.split_once(quote))
                    {
                        names.push(name.to_owned());
                    }
                }
            }
        }
        if let Some(rest) = line.trim_start().strip_prefix("def test_") {
            if let Some(name) = rest.find('(').and_then(|paren| rest.get(..paren)) {
                names.push(format!("test_{name}"));
            }
        }
    }
    names
}

pub(crate) fn coverage_floor_value(source: &str) -> Option<u32> {
    for line in source.lines() {
        for marker in ["fail_under", "--cov-fail-under"] {
            if let Some((_, rest)) = line.split_once(marker) {
                let digits: String = rest
                    .chars()
                    .skip_while(|character| !character.is_ascii_digit())
                    .take_while(|character| character.is_ascii_digit())
                    .collect();
                if let Ok(value) = digits.parse::<u32>() {
                    return Some(value);
                }
            }
        }
    }
    None
}

const DEFERRAL_MARKERS: &[&str] = &[
    "TODO",
    "FIXME",
    concat!("unimplemented", "!"),
    concat!("todo", "!"),
    concat!("raise Not", "ImplementedError"),
    concat!("throw new Error(\"not ", "implemented\")"),
    concat!("throw new Error('not ", "implemented')"),
    concat!("pass  # ", "TODO"),
];

pub(crate) fn find_deferred_marker(line: &str) -> Option<&'static str> {
    DEFERRAL_MARKERS
        .iter()
        .copied()
        .find(|marker| line.contains(marker))
}

pub(crate) fn extract_deferred_annotation(
    line: &str,
) -> Option<Result<(), DeferredAnnotationError>> {
    let rest = line.find("DEFERRED(").and_then(|start| line.get(start..))?;
    Some(parse_deferred_annotation(rest))
}

pub(crate) fn parse_deferred_annotation(raw: &str) -> Result<(), DeferredAnnotationError> {
    let annotation_form_error = || DeferredAnnotationError::NotDeferredForm {
        raw: raw.to_owned(),
    };
    let after_prefix = raw
        .strip_prefix("DEFERRED(")
        .ok_or_else(annotation_form_error)?;
    let (ref_body, after_ref) =
        after_prefix
            .split_once(')')
            .ok_or_else(|| DeferredAnnotationError::MissingOrEmptyRef {
                raw: raw.to_owned(),
            })?;
    let reference = ref_body.strip_prefix('#').unwrap_or(ref_body).trim();
    if reference.is_empty() {
        return Err(DeferredAnnotationError::MissingOrEmptyRef {
            raw: raw.to_owned(),
        });
    }
    let (between, after_bracket) = after_ref.split_once('[').ok_or_else(|| {
        DeferredAnnotationError::MissingOrEmptyRevisit {
            raw: raw.to_owned(),
        }
    })?;
    if !between.trim().is_empty() {
        return Err(DeferredAnnotationError::MissingOrEmptyRevisit {
            raw: raw.to_owned(),
        });
    }
    let (revisit_body, _) = after_bracket.split_once(']').ok_or_else(|| {
        DeferredAnnotationError::MissingOrEmptyRevisit {
            raw: raw.to_owned(),
        }
    })?;
    let revisit_value = revisit_body
        .strip_prefix("revisit:")
        .ok_or_else(|| DeferredAnnotationError::MissingOrEmptyRevisit {
            raw: raw.to_owned(),
        })?
        .trim();
    if revisit_value.is_empty() {
        return Err(DeferredAnnotationError::MissingOrEmptyRevisit {
            raw: raw.to_owned(),
        });
    }
    Ok(())
}

pub(crate) fn ownerset_marker_lines(source: &str) -> Vec<String> {
    source
        .lines()
        .filter(|line| line.contains("(owner-set"))
        .map(|line| line.trim().to_owned())
        .collect()
}
