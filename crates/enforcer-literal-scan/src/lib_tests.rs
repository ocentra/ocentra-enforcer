use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::lexer_c_like::lex_c_like;
use crate::lexer_python::lex_python;
use crate::lexer_rust::lex_rust;
use crate::risk::classify_literal;

#[test]
fn rust_lexer_skips_comments_chars_and_lifetimes() {
    let source = r##"
// "comment"
let c = 'x';
let s = "device.connected";
let raw = r#"/api/devices"#;
fn f<'a>(x: &'a str) {}
"##;
    let literals = lex_rust(source);
    let texts = literals
        .iter()
        .map(|lit| lit.text.as_str())
        .collect::<Vec<_>>();
    assert!(texts.contains(&"device.connected"));
    assert!(texts.contains(&"/api/devices"));
    assert!(!texts.contains(&"comment"));
    assert!(!texts.contains(&"x"));
}

#[test]
fn ts_lexer_classifies_import_specifier() {
    let spec = detect_language(Path::new("x.ts"), false).unwrap();
    let literals = lex_c_like(
        "import x from './x';\nconst s = 'active';",
        spec,
        "x.ts",
        true,
    );
    assert_eq!(literals[0].kind, LiteralKind::ImportSpecifier);
    assert_eq!(literals[1].text, "active");
}

#[test]
fn python_lexer_extracts_fstrings_and_docstrings() {
    let source = "\"\"\"module doc\"\"\"\nvalue = f'user.{kind}'\n# 'comment'\n";
    let literals = lex_python(source);
    assert!(literals
        .iter()
        .any(|lit| lit.kind == LiteralKind::DocString));
    assert!(literals.iter().any(|lit| lit.kind == LiteralKind::FString));
    assert!(!literals.iter().any(|lit| lit.text == "comment"));
}

#[test]
fn scoring_marks_domain_event_high_and_test_fixture_low() {
    let domain = LiteralCandidate {
        text: "device.connected".to_string(),
        line: 1,
        column: 1,
        kind: LiteralKind::Normal,
        context: "let x = \"device.connected\";".to_string(),
    };
    let risk = classify_literal(
        &domain,
        "src/domain/events.rs",
        "rust",
        FileRole::Domain,
        1,
        None,
    );
    assert!(risk.score >= 70, "expected high risk, got {}", risk.score);
    assert_eq!(risk.category, RiskCategory::EventOrCommandName);

    let test = classify_literal(
        &domain,
        "tests/events.test.ts",
        "typescript",
        FileRole::Test,
        1,
        None,
    );
    assert!(test.score < risk.score);
    assert_eq!(test.category, RiskCategory::TestFixture);
}

#[test]
fn secret_is_blocking() {
    let candidate = LiteralCandidate {
        text: "fakeLiteralSecret_ABCDEF1234567890_abcdef".to_string(),
        line: 1,
        column: 1,
        kind: LiteralKind::Normal,
        context: "const key = \"fakeLiteralSecret_...\";".to_string(),
    };
    let finding = classify_literal(
        &candidate,
        "src/config.ts",
        "typescript",
        FileRole::Config,
        1,
        None,
    );
    assert_eq!(finding.category, RiskCategory::SecretLike);
    assert!(finding.blocking);
    assert_eq!(finding.literal_preview, "[REDACTED]");
}

#[test]
fn markdown_is_common_text_not_code() {
    let spec = detect_language(Path::new("README.md"), false).unwrap();
    assert_eq!(spec.family, LanguageFamily::CommonText);
}

#[test]
fn gitignore_and_default_ignored_dirs_work() {
    let root = temp_dir("literal_scan_ignore");
    fs::create_dir_all(root.join("dist")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join(".gitignore"), "ignored.rs\n").unwrap();
    fs::write(root.join("dist/bad.rs"), "let x = \"device.connected\";").unwrap();
    fs::write(root.join("ignored.rs"), "let x = \"device.connected\";").unwrap();
    fs::write(root.join("src/good.rs"), "let x = \"device.connected\";").unwrap();
    let opts = CliOptions {
        root: root.clone(),
        include_low: true,
        ..CliOptions::default()
    };
    let report = run_scan(&opts).unwrap();
    assert_eq!(report.summary.files_scanned, 1);
    assert!(report.ignored.default_dirs >= 1);
    assert!(report.ignored.gitignore >= 1);
    let _ = fs::remove_dir_all(root);
}

fn temp_dir(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    path.push(format!("{name}_{nanos}"));
    fs::create_dir_all(&path).unwrap();
    path
}
