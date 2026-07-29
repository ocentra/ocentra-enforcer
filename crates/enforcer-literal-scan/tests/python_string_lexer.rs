use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use enforcer_literal_scan::{run_scan, CliOptions};

fn test_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = std::env::temp_dir().join(format!("literal_scan_python_string_{nanos}"));
    fs::create_dir_all(&root)?;
    Ok(root)
}

#[test]
fn python_string_lexer_handles_closed_and_unterminated_f_triples_without_panicking(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = test_root()?;
    fs::write(root.join("closed.py"), "value = f\"\"\"live {name}\"\"\"\n")?;
    fs::write(
        root.join("unterminated.py"),
        "value = f\"\"\"unterminated\n",
    )?;
    let opts = CliOptions {
        root: root.clone().into(),
        include_low: true.into(),
        min_score: enforcer_domain::scan_types::LiteralRiskScore::ZERO,
        ..CliOptions::default()
    };

    let report = run_scan(&opts)?;

    assert_eq!(report.summary.files_scanned, 2);
    assert_eq!(report.summary.literals_found, 2);
    fs::remove_dir_all(root)?;
    Ok(())
}
