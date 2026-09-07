use enforcer_syntax::parsers::{classify, Language};

#[test]
fn classify_recognizes_rust_typescript_python_config() {
    assert_eq!(classify("src/main.rs"), Language::Rust);
    // `.tsx` is Language::Tsx, a distinct row from plain TypeScript as of
    // language-parity wave G2.1a (mirrors the baseline's own
    // CBM_LANG_TSX/CBM_LANG_TYPESCRIPT split) -- see `Language::Tsx`'s
    // own doc comment in `src/parsers/mod.rs`.
    assert_eq!(classify("src/App.tsx"), Language::Tsx);
    assert_eq!(classify("src/App.ts"), Language::TypeScript);
    assert_eq!(classify("scripts/build.js"), Language::JavaScript);
    assert_eq!(classify("app/main.py"), Language::Python);
    assert_eq!(classify("Cargo.toml"), Language::Toml);
    assert_eq!(classify("package.json"), Language::Json);
    assert_eq!(classify("ci.yml"), Language::Yaml);
}

#[test]
fn classify_unknown_extension_is_text_only() {
    assert_eq!(classify("NOTES.qux"), Language::TextOnly);
    assert_eq!(classify("no_extension_at_all"), Language::TextOnly);
}
