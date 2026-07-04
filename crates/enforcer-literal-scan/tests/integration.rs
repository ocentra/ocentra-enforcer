#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use enforcer_literal_scan::{run_scan, CliOptions, RiskCategory};

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
}

#[test]
fn bad_dataset_produces_hard_and_soft_findings() {
    let opts = CliOptions {
        root: fixture("bad"),
        include_low: true,
        min_score: 0,
        ..CliOptions::default()
    };
    let report = run_scan(&opts).expect("scan should run");
    assert!(!report.ok, "secret fixture should hard fail");
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
}

#[test]
fn good_dataset_has_no_hard_findings() {
    let opts = CliOptions {
        root: fixture("good"),
        include_low: true,
        min_score: 0,
        ..CliOptions::default()
    };
    let report = run_scan(&opts).expect("scan should run");
    assert!(report.ok, "good fixture should not hard fail");
    assert!(report.hard_findings.is_empty());
}

#[test]
fn many_language_dataset_scans_broad_language_families() {
    let opts = CliOptions {
        root: fixture("languages"),
        include_low: true,
        min_score: 0,
        ..CliOptions::default()
    };
    let report = run_scan(&opts).expect("scan should run");
    assert!(report.ok, "language fixture should have no hard secrets");
    let languages: BTreeSet<_> = report.languages.keys().cloned().collect();
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
}

#[test]
fn ignored_files_are_skipped_by_default() {
    let opts = CliOptions {
        root: fixture("ignored"),
        include_low: true,
        min_score: 0,
        ..CliOptions::default()
    };
    let report = run_scan(&opts).expect("scan should run");
    assert_eq!(report.summary.files_scanned, 0);
    assert!(report.ignored.gitignore >= 1);
    assert!(report.ignored.default_dirs >= 1);
}

#[test]
fn include_ignored_scans_ignored_dataset() {
    let opts = CliOptions {
        root: fixture("ignored"),
        include_ignored: true,
        include_low: true,
        min_score: 0,
        ..CliOptions::default()
    };
    let report = run_scan(&opts).expect("scan should run");
    assert!(report.summary.files_scanned >= 2);
    assert!(report
        .literal_risks
        .iter()
        .any(|finding| finding.literal_preview.contains("ignored.created")));
}

#[test]
fn output_is_deterministic_for_same_input() {
    let opts = CliOptions {
        root: fixture("bad"),
        include_low: true,
        min_score: 0,
        ..CliOptions::default()
    };
    let a = run_scan(&opts).expect("first scan").to_json_pretty();
    let b = run_scan(&opts).expect("second scan").to_json_pretty();
    // Normalize duration because timing is intentionally reported.
    assert_eq!(strip_duration(&a), strip_duration(&b));
}

#[test]
fn fail_above_turns_high_risk_into_hard_failure() {
    let opts = CliOptions {
        root: fixture("bad"),
        files: vec![PathBuf::from("rust_domain.rs")],
        include_low: true,
        min_score: 0,
        fail_above: Some(60),
        ..CliOptions::default()
    };
    let report = run_scan(&opts).expect("scan should run");
    assert!(!report.ok);
    assert!(report
        .hard_findings
        .iter()
        .any(|finding| finding.rule_id.starts_with("LIT-")));
}

#[test]
fn binary_file_does_not_crash() {
    let root = temp_dir("literal_scan_binary");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/bin.rs"), [0u8, 159, 146, 150]).unwrap();
    let opts = CliOptions {
        root: root.clone(),
        ..CliOptions::default()
    };
    let report = run_scan(&opts).expect("scan should handle binary");
    assert_eq!(report.summary.files_scanned, 0);
    assert!(report.ignored.binary >= 1);
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
