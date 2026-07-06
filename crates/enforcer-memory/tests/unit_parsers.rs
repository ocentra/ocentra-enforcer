use enforcer_memory::parsers::{classify, Language};

#[test]
fn classify_recognizes_rust_typescript_python_config() {
    assert_eq!(classify("src/main.rs"), Language::Rust);
    assert_eq!(classify("src/App.tsx"), Language::TypeScript);
    assert_eq!(classify("scripts/build.js"), Language::JavaScript);
    assert_eq!(classify("app/main.py"), Language::Python);
    assert_eq!(classify("Cargo.toml"), Language::ConfigToml);
    assert_eq!(classify("package.json"), Language::ConfigJson);
    assert_eq!(classify("ci.yml"), Language::ConfigYaml);
}

#[test]
fn classify_unknown_extension_is_text_only() {
    assert_eq!(classify("NOTES.qux"), Language::TextOnly);
    assert_eq!(classify("no_extension_at_all"), Language::TextOnly);
}
