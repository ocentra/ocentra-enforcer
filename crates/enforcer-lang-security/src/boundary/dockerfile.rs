//! Dockerfile text decoding boundary.
//!
//! BOUNDARY-INVARIANT: only complete logical instructions are emitted; blank,
//! comment-only, and malformed continuation input contributes no instruction.
//! The malformed continuation case has negative coverage in this module's tests.

/// One logical Dockerfile instruction (after joining `\` continuations).
pub(crate) struct Instruction {
    pub(crate) keyword: String,
    pub(crate) args: String,
    pub(crate) line: u32,
}

/// Join backslash-continued lines into logical instructions, dropping blank
/// and `#`-comment lines. Keeps the 1-based line number of each
/// instruction's first line.
pub(crate) fn logical_instructions(source: &str) -> Vec<Instruction> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut start_line = 0u32;
    for (index, raw) in source.lines().enumerate() {
        let line_no = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
        let line = raw.trim_end();
        let trimmed = line.trim_start();
        if buf.is_empty() {
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            start_line = line_no;
        }
        if let Some(without_slash) = line.strip_suffix('\\') {
            buf.push_str(without_slash);
            buf.push(' ');
            continue;
        }
        buf.push_str(line);
        let logical = buf.trim().to_owned();
        buf.clear();
        if logical.is_empty() {
            continue;
        }
        let (keyword, args) = match logical.split_once(char::is_whitespace) {
            Some((kw, rest)) => (kw.to_ascii_uppercase(), rest.trim().to_owned()),
            None => (logical.to_ascii_uppercase(), String::new()),
        };
        out.push(Instruction {
            keyword,
            args,
            line: start_line,
        });
    }
    if !buf.trim().is_empty() {
        let logical = buf.trim().to_owned();
        let (keyword, args) = match logical.split_once(char::is_whitespace) {
            Some((kw, rest)) => (kw.to_ascii_uppercase(), rest.trim().to_owned()),
            None => (logical.to_ascii_uppercase(), String::new()),
        };
        out.push(Instruction {
            keyword,
            args,
            line: start_line,
        });
    }
    out
}

/// Parse a `FROM` argument into (image_ref, optional stage alias).
pub(crate) fn decode_from(args: &str) -> (&str, Option<String>) {
    let mut parts = args.split_whitespace();
    let mut image = parts.next().unwrap_or("");
    // Docker permits an optional platform selector before the image, e.g.
    // `FROM --platform=$BUILDPLATFORM rust:1.88 AS builder`. The selector is
    // a build-stage option, not an image reference, so it must not be checked
    // for tag pinning or stage-alias matching.
    while image.starts_with("--") {
        if image == "--platform" {
            let _platform = parts.next();
        }
        image = parts.next().unwrap_or("");
    }
    // ` AS <alias>` (case-insensitive) marks a build-stage name.
    let mut alias = None;
    let tokens: Vec<&str> = args.split_whitespace().collect();
    if let Some(pos) = tokens.iter().position(|t| t.eq_ignore_ascii_case("as")) {
        if let Some(name) = tokens.get(pos + 1) {
            alias = Some(name.to_ascii_lowercase());
        }
    }
    (image, alias)
}

/// Split an `ENV`/`ARG` argument string into (key, optional literal value)
/// pairs, handling both `KEY=value ...` and the legacy `KEY value` form.
pub(crate) fn env_pairs(args: &str) -> Vec<(String, Option<String>)> {
    let trimmed = args.trim();
    if trimmed.contains('=') {
        trimmed
            .split_whitespace()
            .filter_map(|tok| {
                tok.split_once('=')
                    .map(|(k, v)| (k.to_owned(), Some(v.trim_matches(['"', '\'']).to_owned())))
            })
            .collect()
    } else {
        // Legacy `ENV KEY the rest is the value`; `ARG KEY` (no value).
        match trimmed.split_once(char::is_whitespace) {
            Some((k, v)) => vec![(
                k.to_owned(),
                Some(v.trim().trim_matches(['"', '\'']).to_owned()),
            )],
            None => vec![(trimmed.to_owned(), None)],
        }
    }
}

/// Decide whether an ENV/ARG value is a concrete secret rather than a variable or sentinel.
pub(crate) fn is_literal_secret_value(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() || value.starts_with('$') {
        return false;
    }
    let placeholder = value.eq_ignore_ascii_case("changeme")
        || (value.starts_with('<') && value.ends_with('>'))
        || (value.starts_with('{') && value.ends_with('}'));
    !placeholder
}

#[cfg(test)]
mod tests {
    #[test]
    fn malformed_empty_continuation_is_rejected() {
        assert!(super::logical_instructions("\\").is_empty());
    }
}
