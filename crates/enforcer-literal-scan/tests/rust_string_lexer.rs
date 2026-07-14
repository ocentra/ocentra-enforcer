use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use enforcer_literal_scan::{run_scan, CliOptions};

fn test_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = std::env::temp_dir().join(format!("literal_scan_rust_string_{nanos}"));
    fs::create_dir_all(&root)?;
    Ok(root)
}

#[test]
fn rust_string_lexer_keeps_closed_raw_strings_and_handles_unterminated_ones(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = test_root()?;
    fs::write(
        root.join("closed.rs"),
        "const VALUE: &str = r#\"live value\"#;\n",
    )?;
    fs::write(
        root.join("unterminated.rs"),
        "const VALUE: &str = r#\"unterminated\n",
    )?;
    let opts = CliOptions {
        root: root.clone(),
        include_low: true,
        min_score: 0,
        ..CliOptions::default()
    };

    let report = run_scan(&opts)?;

    assert_eq!(report.summary.files_scanned, 2);
    assert_eq!(report.summary.literals_found, 1);
    fs::remove_dir_all(root)?;
    Ok(())
}
