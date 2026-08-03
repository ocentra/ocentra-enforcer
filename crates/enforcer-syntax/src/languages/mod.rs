//! Per-language tree-sitter extraction. Each submodule owns one
//! grammar and turns its parse tree into the language-agnostic
//! [`crate::parsers::ParsedFile`] shape that [`crate::code_graph`]
//! turns into graph nodes/edges. Adding a new language means adding a
//! new submodule here plus one dispatch arm in
//! [`crate::parsers::parse_file`] -- [`crate::code_graph`] itself never
//! needs to know which grammar produced a [`crate::parsers::ParsedFile`].

pub mod c;
pub mod cpp;
pub mod csharp;
pub mod generic;
pub mod go;
pub mod java;
pub mod php;
pub mod python;
pub mod rust;
pub mod spec;
pub mod typescript;

/// Tree-sitter's native scanners are not safe for binary/control input.
/// Reject embedded NULs, non-whitespace control characters, and Unicode
/// format/bidi controls before any language-specific parser crosses that ABI
/// boundary. Callers keep the total-parser contract by returning an empty
/// parsed file.
pub(crate) fn has_unsafe_tree_sitter_input(source: &str) -> bool {
    source.chars().any(|character| {
        (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
            || is_tree_sitter_format_character(character)
    })
}

/// The Bash, Just, and Odin native grammar paths are unsafe for
/// supplementary-plane code points. Keep this narrower than the shared ABI
/// guard: grammars such as Rust legitimately parse astral Unicode in string
/// literals and function bodies.
pub(crate) fn has_unsafe_external_scanner_input(source: &str) -> bool {
    source.chars().any(|character| character > '\u{FFFF}')
}

/// Unicode `Cf` format controls are not visible source text and are unsafe to
/// hand to third-party tree-sitter scanners. Keep this explicit and
/// dependency-free so the ABI guard applies uniformly to every vendored and
/// crates.io grammar.
fn is_tree_sitter_format_character(character: char) -> bool {
    matches!(
        character,
        '\u{00AD}'
            | '\u{061C}'
            | '\u{06DD}'
            | '\u{070F}'
            | '\u{0890}'..='\u{0891}'
            | '\u{08E2}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{E0001}'
            | '\u{E0020}'..='\u{E007F}'
    )
}
