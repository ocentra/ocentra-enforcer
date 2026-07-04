use super::{LanguageFamily, LanguageSpec, LiteralCandidate};
use crate::lexer_c_like::{lex_c_like, lex_hash_comment, lex_lisp, lex_markup, lex_shell};
use crate::lexer_python::lex_python;
use crate::lexer_rust::lex_rust;
pub(crate) use crate::lexer_shared::line_at;
pub(crate) fn lex_literals(
    source: &str,
    language: LanguageSpec,
    rel: &str,
) -> Vec<LiteralCandidate> {
    match language.family {
        LanguageFamily::Rust => lex_rust(source),
        LanguageFamily::TypeScript => lex_c_like(source, language, rel, true),
        LanguageFamily::Python => lex_python(source),
        LanguageFamily::CLike => lex_c_like(source, language, rel, false),
        LanguageFamily::HashComment => lex_hash_comment(source, language),
        LanguageFamily::Shell => lex_shell(source),
        LanguageFamily::Lisp => lex_lisp(source),
        LanguageFamily::Markup => lex_markup(source),
        LanguageFamily::Fallback => lex_c_like(source, language, rel, false),
        LanguageFamily::CommonText | LanguageFamily::Sql => Vec::new(),
    }
}
