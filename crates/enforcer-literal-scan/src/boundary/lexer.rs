use super::{LanguageFamily, LanguageSpec, LiteralCandidate};
use crate::lexer_c_like_scan::lex_c_like;
use crate::lexer_hash_comment_scan::lex_hash_comment;
use crate::lexer_lisp_scan::lex_lisp;
use crate::lexer_markup_scan::lex_markup;
use crate::lexer_python_scan::lex_python;
use crate::lexer_rust_scan::lex_rust;
use crate::lexer_shell_scan::lex_shell;
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
