//! Raw TypeScript source-shape parsing owned by the analyzer boundary.
//!
//! BOUNDARY-INVARIANT: raw source fragments are reduced to typed analyzer
//! observations before validators make policy decisions.
//! boundaryOwnerNote: enforcer-lang-ts owns source-shape boundary parsing.
//! Negative malformed and truncated source coverage is fixture-backed.

use enforcer_domain::paths::RelPath;

/// Extract the quoted module target from an import/export-from statement.
pub(crate) fn import_target(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("import ") && !trimmed.starts_with("export ") {
        return None;
    }
    let (_, rest) = trimmed.split_once(" from ")?;
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let quoted = rest.strip_prefix(quote)?;
    let (target, _) = quoted.split_once(quote)?;
    Some(target)
}

/// Read the owning feature segment from a repository-relative path.
pub(crate) fn owning_feature(path: &RelPath) -> Option<&str> {
    let (_, rest) = path.as_str().split_once("features/")?;
    let (feature, _) = rest.split_once('/')?;
    (!feature.is_empty()).then_some(feature)
}

/// Read an aliased feature segment from `@/features/<name>/...`.
pub(crate) fn aliased_imported_feature(target: &str) -> Option<&str> {
    let (_, rest) = target.split_once("@/features/")?;
    let feature = rest.split('/').next()?;
    (!feature.is_empty()).then_some(feature)
}

/// Read an aliased or relative deep-import feature segment.
pub(crate) fn layered_imported_feature(target: &str) -> Option<&str> {
    if let Some(feature) = aliased_imported_feature(target) {
        return Some(feature);
    }
    let stripped = target.strip_prefix("../")?;
    let (name, remainder) = stripped.split_once('/')?;
    if !name.is_empty() && remainder.contains("internal/") {
        return Some(name);
    }
    None
}

/// Classify whether a repository path belongs to a router layer.
pub(crate) fn in_router_layer(path: &RelPath) -> bool {
    let path = path.as_str();
    path.contains("/router/")
        || path.contains("/routers/")
        || path.starts_with("router/")
        || path.starts_with("routers/")
}

/// Detect a bare `: any` annotation at a right-hand word boundary.
pub(crate) fn is_any_annotation(text: &str) -> bool {
    let Some(idx) = text.find(": any") else {
        return false;
    };
    let Some(end) = idx.checked_add(": any".len()) else {
        return false;
    };
    text.get(end..)
        .and_then(|suffix| suffix.chars().next())
        .is_none_or(|character| !(character.is_alphanumeric() || character == '_'))
}

/// Parse brace-import names from a non-type-only import statement.
pub(crate) fn brace_import_names(line: &str) -> Option<Vec<&str>> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("import ") || trimmed.starts_with("import type ") {
        return None;
    }
    let open = trimmed.find('{')?;
    let close = trimmed.find('}')?;
    if close < open {
        return None;
    }
    Some(
        trimmed
            .get(open.checked_add(1)?..close)?
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .collect(),
    )
}

/// Classify whether an enum member has a string-literal initializer.
pub(crate) fn member_has_string_initializer(text: &str) -> bool {
    let Some((_, rhs)) = text.split_once('=') else {
        return false;
    };
    let rhs = rhs.trim().trim_end_matches(',');
    (rhs.starts_with('\'') && rhs.ends_with('\'') && rhs.len() >= 2)
        || (rhs.starts_with('"') && rhs.ends_with('"') && rhs.len() >= 2)
}

/// Classify whether a dependency-injection token is symbolic.
pub(crate) fn is_symbol_di_token(token: &str) -> bool {
    let token = token.trim();
    token.starts_with("Symbol")
        || token.ends_with("Token")
        || token.ends_with("Symbol")
        || (token.starts_with('I')
            && token.len() > 1
            && token.chars().nth(1).is_some_and(char::is_uppercase))
}

/// Detect a skipped, focused, or todo JavaScript test suite.
pub(crate) fn has_skip_or_only_suite_modifier(text: &str) -> bool {
    ["describe", "test", "it"].iter().any(|base| {
        [".skip(", ".only(", ".todo("]
            .iter()
            .any(|suffix| text.contains(&format!("{base}{suffix}")))
    })
}

/// Detect deliberately weak assertion shapes.
pub(crate) fn has_weak_assertion(text: &str) -> bool {
    text.contains("expect(true).toBe(true)") || text.contains(".toBeTruthy()")
}

/// Detect whether a test source exercises a decoder or schema.
pub(crate) fn exercises_a_decoder(source: &str) -> bool {
    source.contains("Schema.decode")
        || source.contains("decodeUnknown")
        || source.contains("Schema.Struct")
}

/// Detect whether a decoder test source contains a negative case.
pub(crate) fn has_negative_case(source: &str) -> bool {
    source.contains("toThrow")
        || source.contains(".rejects")
        || source.contains("invalid")
        || source.contains("malformed")
}
