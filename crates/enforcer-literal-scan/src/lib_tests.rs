use super::{
    run_scan, CliOptions, FileRole, LanguageFamily, LiteralCandidate, LiteralKind, RiskCategory,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use enforcer_domain::scan_types::{
    LiteralFindingPath, LiteralLanguageId, LiteralScanToggle, LiteralSourceColumn,
    LiteralSourceLine,
};

use crate::lexer_c_like_scan::lex_c_like;
use crate::lexer_python_scan::lex_python;
use crate::lexer_rust_scan::lex_rust;
use crate::risk::{classify_literal, ClassificationInput};

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
fn ts_lexer_classifies_import_specifier() -> Result<(), Box<dyn std::error::Error>> {
    let spec = crate::language_registry::detect_language(Path::new("x.ts"), false)
        .ok_or_else(|| std::io::Error::other("TypeScript language must be registered"))?;
    let literals = lex_c_like(
        "import x from './x';\nconst s = 'active';",
        spec,
        "x.ts",
        true,
    );
    let import = literals
        .first()
        .ok_or_else(|| std::io::Error::other("import literal must exist"))?;
    let active = literals
        .get(1)
        .ok_or_else(|| std::io::Error::other("active literal must exist"))?;
    assert_eq!(import.kind, LiteralKind::ImportSpecifier);
    assert_eq!(active.text, "active");
    Ok(())
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
fn scoring_marks_domain_event_high_and_test_fixture_low() -> Result<(), Box<dyn std::error::Error>>
{
    let domain = LiteralCandidate {
        text: "device.connected".to_string().into(),
        line: LiteralSourceLine::from_one_based(1),
        column: LiteralSourceColumn::from_one_based(1),
        kind: LiteralKind::Normal,
        context: "let x = \"device.connected\";".to_string().into(),
    };
    let domain_file = LiteralFindingPath::try_new("src/domain/events.rs".to_string())?;
    let rust_language: LiteralLanguageId = "rust".parse()?;
    let risk = classify_literal(ClassificationInput {
        candidate: &domain,
        file: &domain_file,
        language: &rust_language,
        role: FileRole::Domain,
        repeated_files: 1,
        fail_above: None,
    });
    assert!(risk.score >= 70, "expected high risk, got {}", risk.score);
    assert_eq!(risk.category, RiskCategory::EventOrCommandName);

    let test_file = LiteralFindingPath::try_new("tests/events.test.ts".to_string())?;
    let typescript_language: LiteralLanguageId = "typescript".parse()?;
    let test = classify_literal(ClassificationInput {
        candidate: &domain,
        file: &test_file,
        language: &typescript_language,
        role: FileRole::Test,
        repeated_files: 1,
        fail_above: None,
    });
    assert!(test.score < risk.score);
    assert_eq!(test.category, RiskCategory::TestFixture);
    Ok(())
}

#[test]
fn secret_is_blocking() -> Result<(), Box<dyn std::error::Error>> {
    let candidate = LiteralCandidate {
        text: "fakeLiteralSecret_ABCDEF1234567890_abcdef"
            .to_string()
            .into(),
        line: LiteralSourceLine::from_one_based(1),
        column: LiteralSourceColumn::from_one_based(1),
        kind: LiteralKind::Normal,
        context: "const key = \"fakeLiteralSecret_...\";".to_string().into(),
    };
    let file = LiteralFindingPath::try_new("src/config.ts".to_string())?;
    let language: LiteralLanguageId = "typescript".parse()?;
    let finding = classify_literal(ClassificationInput {
        candidate: &candidate,
        file: &file,
        language: &language,
        role: FileRole::Config,
        repeated_files: 1,
        fail_above: None,
    });
    assert_eq!(finding.category, RiskCategory::SecretLike);
    assert!(finding.blocking.is_blocking());
    assert_eq!(finding.literal_preview, "[REDACTED]");
    Ok(())
}

#[test]
fn markdown_is_common_text_not_code() -> Result<(), Box<dyn std::error::Error>> {
    let spec = crate::language_registry::detect_language(Path::new("README.md"), false)
        .ok_or_else(|| std::io::Error::other("Markdown language must be registered"))?;
    assert_eq!(spec.family, LanguageFamily::CommonText);
    Ok(())
}

#[test]
fn gitignore_and_default_ignored_dirs_work() -> Result<(), Box<dyn std::error::Error>> {
    let root = temp_dir("literal_scan_ignore")?;
    fs::create_dir_all(root.join("dist"))?;
    fs::create_dir_all(root.join("src"))?;
    fs::write(root.join(".gitignore"), "ignored.rs\n")?;
    fs::write(root.join("dist/bad.rs"), "let x = \"device.connected\";")?;
    fs::write(root.join("ignored.rs"), "let x = \"device.connected\";")?;
    fs::write(root.join("src/good.rs"), "let x = \"device.connected\";")?;
    let opts = CliOptions {
        root: root.clone().into(),
        include_low: LiteralScanToggle::Enabled,
        ..CliOptions::default()
    };
    let report = run_scan(&opts)?;
    assert_eq!(report.summary.files_scanned, 1);
    assert!(report.ignored.default_dirs >= 1);
    assert!(report.ignored.gitignore >= 1);
    let _ = fs::remove_dir_all(root);
    Ok(())
}

fn temp_dir(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    path.push(format!("{name}_{nanos}"));
    fs::create_dir_all(&path)?;
    Ok(path)
}
