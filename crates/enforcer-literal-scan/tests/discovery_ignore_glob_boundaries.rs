use std::path::PathBuf;

use enforcer_literal_scan::{run_scan, CliOptions};

fn isolated_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "enforcer-literal-scan-glob-boundaries-{}",
        std::process::id()
    ))
}

#[test]
fn wildcard_gitignore_matches_empty_and_nonempty_path_suffixes_without_panicking(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = isolated_root();
    if root.exists() {
        std::fs::remove_dir_all(&root)?;
    }
    std::fs::create_dir_all(&root)?;
    std::fs::write(root.join(".gitignore"), "cache/*\n")?;
    std::fs::create_dir_all(root.join("cache"))?;
    std::fs::write(
        root.join("cache/entry.rs"),
        "let event = \"cache.created\";\n",
    )?;

    let report = run_scan(&CliOptions {
        root: root.clone(),
        include_low: true,
        min_score: 0,
        ..CliOptions::default()
    })?;

    std::fs::remove_dir_all(&root)?;
    assert_eq!(report.summary.files_scanned, 0);
    assert_eq!(report.ignored.gitignore, 1);
    Ok(())
}
