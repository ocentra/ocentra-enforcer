use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use enforcer_literal_scan::{run_scan, CliOptions};

fn test_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = std::env::temp_dir().join(format!("literal_scan_lisp_{nanos}"));
    fs::create_dir_all(&root)?;
    Ok(root)
}

#[test]
fn lisp_lexer_ignores_comments_and_unterminated_literals() -> Result<(), Box<dyn std::error::Error>>
{
    let root = test_root()?;
    fs::write(
        root.join("sample.clj"),
        "; \"comment-only-literal\"\n(def label \"live-literal\")\n\"unterminated",
    )?;
    let opts = CliOptions {
        root: root.clone(),
        include_low: true,
        min_score: 0,
        ..CliOptions::default()
    };

    let report = run_scan(&opts)?;

    assert_eq!(report.summary.files_scanned, 1);
    assert_eq!(report.summary.literals_found, 1);
    fs::remove_dir_all(root)?;
    Ok(())
}
