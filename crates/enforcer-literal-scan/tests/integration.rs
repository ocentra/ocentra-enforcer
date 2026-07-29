use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use enforcer_domain::findings::ReportOutcome;
use enforcer_domain::scan_types::LiteralRiskCategory as RiskCategory;
use enforcer_literal_scan::{run_scan, CliOptions};

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
}

#[test]
fn bad_dataset_produces_hard_and_soft_findings() -> Result<(), Box<dyn std::error::Error>> {
    let opts = CliOptions {
        root: fixture("bad").into(),
        include_low: true.into(),
        min_score: enforcer_domain::scan_types::LiteralRiskScore::ZERO,
        ..CliOptions::default()
    };
    let report = run_scan(&opts)?;
    assert_eq!(report.ok, ReportOutcome::Violations);
    assert!(report
        .hard_findings
        .iter()
        .any(|finding| finding.category == RiskCategory::SecretLike));
    assert!(report
        .literal_risks
        .iter()
        .any(|finding| finding.category == RiskCategory::EventOrCommandName));
    assert!(report
        .literal_risks
        .iter()
        .any(|finding| finding.category == RiskCategory::RouteOrUrl));
    assert!(report
        .literal_risks
        .iter()
        .any(|finding| finding.category == RiskCategory::MagicStringComparison));
    Ok(())
}

#[test]
fn good_dataset_has_no_hard_findings() -> Result<(), Box<dyn std::error::Error>> {
    let opts = CliOptions {
        root: fixture("good").into(),
        include_low: true.into(),
        min_score: enforcer_domain::scan_types::LiteralRiskScore::ZERO,
        ..CliOptions::default()
    };
    let report = run_scan(&opts)?;
    assert_eq!(report.ok, ReportOutcome::Clean);
    assert!(report.hard_findings.is_empty());
    Ok(())
}

#[test]
fn many_language_dataset_scans_broad_language_families() -> Result<(), Box<dyn std::error::Error>> {
    let opts = CliOptions {
        root: fixture("languages").into(),
        include_low: true.into(),
        min_score: enforcer_domain::scan_types::LiteralRiskScore::ZERO,
        ..CliOptions::default()
    };
    let report = run_scan(&opts)?;
    assert_eq!(report.ok, ReportOutcome::Clean);
    let languages: BTreeSet<_> = report.languages.keys().map(|key| key.as_str()).collect();
    for expected in [
        "c",
        "cpp",
        "objective-c",
        "go",
        "java",
        "csharp",
        "kotlin",
        "scala",
        "groovy",
        "swift",
        "dart",
        "d",
        "v",
        "solidity",
        "move",
        "apex",
        "qml",
        "cuda",
        "shader",
        "php",
        "ruby",
        "perl",
        "lua",
        "r",
        "julia",
        "raku",
        "shell",
        "powershell",
        "batch",
        "haskell",
        "ocaml",
        "reason",
        "rescript",
        "sml",
        "fsharp",
        "elixir",
        "erlang",
        "clojure",
        "lisp",
        "zig",
        "nim",
        "starlark",
        "nix",
        "protobuf",
        "thrift",
        "dockerfile",
        "html",
    ] {
        assert!(
            languages.contains(expected),
            "expected language {expected} in {languages:?}"
        );
    }
    assert!(
        !languages.contains("markdown"),
        "markdown must not be code literal-risk scanned"
    );
    assert!(
        !languages.contains("json"),
        "json must not be code literal-risk scanned"
    );
    assert!(report
        .literal_risks
        .iter()
        .any(|finding| finding.category == RiskCategory::ShellFragment));
    assert!(report
        .literal_risks
        .iter()
        .any(|finding| finding.category == RiskCategory::ProtocolHeaderOrMedia));
    assert!(report
        .literal_risks
        .iter()
        .any(|finding| finding.category == RiskCategory::EventOrCommandName));
    Ok(())
}

#[test]
fn ignored_files_are_skipped_by_default() -> Result<(), Box<dyn std::error::Error>> {
    let opts = CliOptions {
        root: fixture("ignored").into(),
        include_low: true.into(),
        min_score: enforcer_domain::scan_types::LiteralRiskScore::ZERO,
        ..CliOptions::default()
    };
    let report = run_scan(&opts)?;
    assert_eq!(report.summary.files_scanned, 0);
    assert!(report.ignored.gitignore >= 1);
    assert!(report.ignored.default_dirs >= 1);
    Ok(())
}

#[test]
fn include_ignored_scans_ignored_dataset() -> Result<(), Box<dyn std::error::Error>> {
    let opts = CliOptions {
        root: fixture("ignored").into(),
        include_ignored: true.into(),
        include_low: true.into(),
        min_score: enforcer_domain::scan_types::LiteralRiskScore::ZERO,
        ..CliOptions::default()
    };
    let report = run_scan(&opts)?;
    assert!(report.summary.files_scanned >= 2);
    assert!(report
        .literal_risks
        .iter()
        .any(|finding| finding.literal_preview.contains("ignored.created")));
    Ok(())
}

#[test]
fn output_is_deterministic_for_same_input() -> Result<(), Box<dyn std::error::Error>> {
    let opts = CliOptions {
        root: fixture("bad").into(),
        include_low: true.into(),
        min_score: enforcer_domain::scan_types::LiteralRiskScore::ZERO,
        ..CliOptions::default()
    };
    let a = run_scan(&opts)?.to_json_pretty();
    let b = run_scan(&opts)?.to_json_pretty();
    // Normalize duration because timing is intentionally reported.
    assert_eq!(strip_duration(a.as_str()), strip_duration(b.as_str()));
    Ok(())
}

#[test]
fn fail_above_turns_high_risk_into_hard_failure() -> Result<(), Box<dyn std::error::Error>> {
    let opts = CliOptions {
        root: fixture("bad").into(),
        files: vec![PathBuf::from("rust_domain.rs")].into(),
        include_low: true.into(),
        min_score: enforcer_domain::scan_types::LiteralRiskScore::ZERO,
        fail_above: Some(enforcer_domain::scan_types::LiteralRiskScore::HIGH_RISK_THRESHOLD),
        ..CliOptions::default()
    };
    let report = run_scan(&opts)?;
    assert_eq!(report.ok, ReportOutcome::Violations);
    assert!(report
        .hard_findings
        .iter()
        .any(|finding| finding.rule_id.as_str().starts_with("LIT-")));
    Ok(())
}

#[test]
fn hash_comment_lexer_ignores_comments_and_keeps_closed_triple_literals(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = temp_dir("literal_scan_hash_comment")?;
    fs::write(
        root.join("sample.py"),
        "# \"comment-only-literal\"\nvalue = \"\"\"triple-live-value\"\"\"\nlabel = \"normal-live-value\"\n",
    )
    ?;
    let opts = CliOptions {
        root: root.clone().into(),
        include_low: true.into(),
        min_score: enforcer_domain::scan_types::LiteralRiskScore::ZERO,
        ..CliOptions::default()
    };

    let report = run_scan(&opts)?;

    assert_eq!(report.summary.files_scanned, 1);
    assert_eq!(report.summary.literals_found, 2);
    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn binary_file_does_not_crash() -> Result<(), Box<dyn std::error::Error>> {
    let root = temp_dir("literal_scan_binary")?;
    fs::create_dir_all(root.join("src"))?;
    fs::write(root.join("src/bin.rs"), [0u8, 159, 146, 150])?;
    let opts = CliOptions {
        root: root.clone().into(),
        ..CliOptions::default()
    };
    let report = run_scan(&opts)?;
    assert_eq!(report.summary.files_scanned, 0);
    assert!(report.ignored.binary >= 1);
    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn truncated_c_like_string_openers_do_not_crash_the_scan() -> Result<(), Box<dyn std::error::Error>>
{
    let root = temp_dir("literal_scan_truncated_c_like")?;
    fs::write(root.join("sample.cs"), "@\"\n")?;
    fs::write(root.join("sample.cpp"), "\"\"\"unterminated\n")?;
    fs::write(root.join("sample.ts"), "const token = `unterminated\n")?;
    let report = run_scan(&CliOptions {
        root: root.clone().into(),
        include_low: true.into(),
        min_score: enforcer_domain::scan_types::LiteralRiskScore::ZERO,
        ..CliOptions::default()
    })?;
    assert_eq!(report.summary.files_scanned, 3);
    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn truncated_shell_quote_does_not_crash_the_scan() -> Result<(), Box<dyn std::error::Error>> {
    let root = temp_dir("literal_scan_truncated_shell")?;
    fs::write(root.join("script.sh"), "echo 'unterminated\n# comment")?;
    let report = run_scan(&CliOptions {
        root: root.clone().into(),
        include_low: true.into(),
        min_score: enforcer_domain::scan_types::LiteralRiskScore::ZERO,
        ..CliOptions::default()
    })?;
    assert_eq!(report.summary.files_scanned, 1);
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

fn strip_duration(input: &str) -> String {
    input
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("\"durationMs\"") {
                "    \"durationMs\": 0".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
