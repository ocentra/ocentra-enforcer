//! Idempotent managed-block markers for text configs an adapter edits in
//! place (e.g. a `CLAUDE.md` doctrine block, an `AGENTS.md` tool-neutral
//! reference) without clobbering surrounding user content.
//!
//! A managed block is bounded by a begin/end marker pair derived from a
//! caller-supplied name:
//! ```text
//! <!-- enforcer:managed:begin:{name} -->
//! ...content the installer owns...
//! <!-- enforcer:managed:end:{name} -->
//! ```
//! Re-running an adapter replaces only the content between an existing
//! pair (idempotent re-install); a missing pair means the block is
//! appended fresh; a malformed pair (unterminated, or more than one begin/
//! end) is a detected [`crate::error::InstallError::ManagedBlockInvalid`],
//! never a silent overwrite of unrelated file content.

use crate::error::{InstallError, InstallResult};

fn begin_marker(name: &str) -> String {
    format!("<!-- enforcer:managed:begin:{name} -->")
}

fn end_marker(name: &str) -> String {
    format!("<!-- enforcer:managed:end:{name} -->")
}

/// Render a full managed block (markers + content) as a standalone
/// string, for the "no existing block" append case.
#[must_use]
pub fn render_block(name: &str, content: &str) -> String {
    format!(
        "{}\n{}\n{}",
        begin_marker(name),
        content.trim_end(),
        end_marker(name)
    )
}

/// Insert or replace the named managed block inside `existing`. Returns
/// the full new file content. If no block named `name` is present, the
/// rendered block is appended (with a leading blank line if `existing` is
/// non-empty and does not already end in one).
///
/// # Errors
/// Returns [`InstallError::ManagedBlockInvalid`] if the begin/end markers
/// for `name` appear more than once, or a begin marker appears without a
/// matching end marker (or vice versa) — a malformed managed block is
/// reported, never silently patched over.
pub fn upsert_block(
    existing: &str,
    name: &str,
    content: &str,
    path: &str,
) -> InstallResult<String> {
    let begin = begin_marker(name);
    let end = end_marker(name);

    let begin_count = existing.matches(begin.as_str()).count();
    let end_count = existing.matches(end.as_str()).count();

    if begin_count == 0 && end_count == 0 {
        let rendered = render_block(name, content);
        if existing.is_empty() {
            return Ok(rendered);
        }
        let sep = if existing.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        return Ok(format!("{existing}{sep}{rendered}\n"));
    }

    if begin_count != 1 || end_count != 1 {
        return Err(InstallError::ManagedBlockInvalid {
            path: path.to_owned(),
            marker: name.to_owned(),
            reason: format!(
                "expected exactly one begin/end marker pair, found {begin_count} begin and {end_count} end"
            ),
        });
    }

    let begin_idx =
        existing
            .find(begin.as_str())
            .ok_or_else(|| InstallError::ManagedBlockInvalid {
                path: path.to_owned(),
                marker: name.to_owned(),
                reason: "begin marker vanished during re-scan".to_owned(),
            })?;
    let end_idx = existing
        .find(end.as_str())
        .ok_or_else(|| InstallError::ManagedBlockInvalid {
            path: path.to_owned(),
            marker: name.to_owned(),
            reason: "end marker vanished during re-scan".to_owned(),
        })?;

    if end_idx < begin_idx {
        return Err(InstallError::ManagedBlockInvalid {
            path: path.to_owned(),
            marker: name.to_owned(),
            reason: "end marker appears before begin marker".to_owned(),
        });
    }

    let before = &existing[..begin_idx];
    let after = &existing[end_idx + end.len()..];
    let rendered = render_block(name, content);
    Ok(format!("{before}{rendered}{after}"))
}

#[cfg(test)]
mod tests {
    use super::{render_block, upsert_block};

    #[test]
    fn render_block_wraps_content_in_named_markers() {
        let block = render_block("claude-doctrine", "hello");
        assert!(block.starts_with("<!-- enforcer:managed:begin:claude-doctrine -->"));
        assert!(block.ends_with("<!-- enforcer:managed:end:claude-doctrine -->"));
        assert!(block.contains("hello"));
    }

    #[test]
    fn upsert_appends_when_no_existing_block() -> Result<(), Box<dyn std::error::Error>> {
        let out = upsert_block(
            "# My Doc\n\nSome content.\n",
            "doctrine",
            "new stuff",
            "AGENTS.md",
        )?;
        assert!(out.starts_with("# My Doc\n\nSome content.\n"));
        assert!(out.contains("<!-- enforcer:managed:begin:doctrine -->"));
        assert!(out.contains("new stuff"));
        Ok(())
    }

    #[test]
    fn upsert_into_empty_file_yields_just_the_block() -> Result<(), Box<dyn std::error::Error>> {
        let out = upsert_block("", "doctrine", "content", "AGENTS.md")?;
        assert_eq!(out, render_block("doctrine", "content"));
        Ok(())
    }

    #[test]
    fn upsert_replaces_existing_block_content_idempotently(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let first = upsert_block("preamble\n", "doctrine", "v1 content", "AGENTS.md")?;
        let second = upsert_block(&first, "doctrine", "v2 content", "AGENTS.md")?;
        assert!(second.contains("preamble"));
        assert!(second.contains("v2 content"));
        assert!(!second.contains("v1 content"));
        // Exactly one marker pair survives re-application.
        assert_eq!(
            second
                .matches("<!-- enforcer:managed:begin:doctrine -->")
                .count(),
            1
        );
        assert_eq!(
            second
                .matches("<!-- enforcer:managed:end:doctrine -->")
                .count(),
            1
        );
        Ok(())
    }

    #[test]
    fn upsert_preserves_content_outside_the_block() -> Result<(), Box<dyn std::error::Error>> {
        let existing = "before\n<!-- enforcer:managed:begin:doctrine -->\nold\n<!-- enforcer:managed:end:doctrine -->\nafter\n";
        let out = upsert_block(existing, "doctrine", "new", "AGENTS.md")?;
        assert!(out.starts_with("before\n"));
        assert!(out.trim_end().ends_with("after"));
        assert!(out.contains("new"));
        assert!(!out.contains("old"));
        Ok(())
    }

    #[test]
    fn upsert_detects_duplicated_begin_marker_as_malformed() {
        let existing = "<!-- enforcer:managed:begin:doctrine -->\na\n<!-- enforcer:managed:begin:doctrine -->\nb\n<!-- enforcer:managed:end:doctrine -->\n";
        let result = upsert_block(existing, "doctrine", "new", "AGENTS.md");
        assert!(result.is_err());
    }

    #[test]
    fn upsert_detects_missing_end_marker_as_malformed() {
        let existing = "<!-- enforcer:managed:begin:doctrine -->\nunterminated\n";
        let result = upsert_block(existing, "doctrine", "new", "AGENTS.md");
        assert!(result.is_err());
    }

    #[test]
    fn different_named_blocks_do_not_collide() -> Result<(), Box<dyn std::error::Error>> {
        let existing = "<!-- enforcer:managed:begin:a -->\nA\n<!-- enforcer:managed:end:a -->\n";
        let out = upsert_block(existing, "b", "B content", "AGENTS.md")?;
        assert!(out.contains("<!-- enforcer:managed:begin:a -->"));
        assert!(out.contains("A"));
        assert!(out.contains("<!-- enforcer:managed:begin:b -->"));
        assert!(out.contains("B content"));
        Ok(())
    }
}
