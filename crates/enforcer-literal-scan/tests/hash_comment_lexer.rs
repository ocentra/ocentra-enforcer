use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use enforcer_literal_scan::{run_scan, CliOptions};

fn test_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = std::env::temp_dir().join(format!("literal_scan_hash_comment_{nanos}"));
    fs::create_dir_all(&root)?;
    Ok(root)
}

#[test]
fn hash_comment_lexer_ignores_comments_and_keeps_closed_triple_literals(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = test_root()?;
    fs::write(
        root.join("sample.py"),
        "# \"comment-only-literal\"\nvalue = \"\"\"triple-live-value\"\"\"\nlabel = \"normal-live-value\"\n",
    )?;
    let opts = CliOptions {
        root: root.clone(),
        include_low: true,
        min_score: 0,
        ..CliOptions::default()
    };

    let report = run_scan(&opts)?;

    assert_eq!(report.summary.files_scanned, 1);
    assert_eq!(report.summary.literals_found, 2);
    fs::remove_dir_all(root)?;
    Ok(())
}
