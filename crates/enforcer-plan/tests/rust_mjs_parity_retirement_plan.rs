//! Mechanical contract for the Rust/MJS parity-retirement plan.

use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const WORKPACK_IDS: [&str; 15] = [
    "RM00", "RM01", "RM02", "RM03", "RM04", "RM05", "RM06", "RM07", "RM08", "RM09", "RM10", "RM11",
    "RM12", "RM13", "RM14",
];

fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "expected enforcer-plan below the workspace root".into())
}

fn require_all(source: &str, required: &[&str]) {
    for value in required {
        assert!(
            source.contains(value),
            "missing parity-plan marker: {value}"
        );
    }
}

#[test]
fn parity_retirement_plan_has_all_workpacks_and_correct_authority() -> TestResult {
    let root = workspace_root()?;
    let plan = root.join("docs/plans/rust-mjs-parity-retirement-plan");
    let readme = std::fs::read_to_string(plan.join("README.md"))?;
    for required_doc in ["AGENTS.md", "ARCHITECTURE.md", "WORKER_CHECKLIST.md"] {
        assert!(plan.join(required_doc).is_file(), "missing {required_doc}");
    }
    require_all(
        &readme,
        &[
            "planId: rust-mjs-parity-retirement-plan",
            "267af94b701bd592e01a47649e3c18c26ee04239",
            "immutable public oracle",
            "d7162b6173e2c664547fcb9715ba135c435d0b1e",
            "Common fork base only; it is not the current public oracle.",
            "9d21780f9a4f5a498fb16a6b1ae1c05ac2d83e36",
            "allowlist-only",
            "never a public oracle, public source, public verdict input, or merge source",
            "rust-build",
            "267af94b701bd592e01a47649e3c18c26ee04239",
            "split runtime authority",
            "union/equal-or-stricter proof",
            "overlay's two exact allowlisted behaviors",
            "exact-SHA aggregate",
            "native rollback rehearsal",
            "delete-not-merge retirement",
            "never a production fallback",
        ],
    );
    let architecture = std::fs::read_to_string(plan.join("ARCHITECTURE.md"))?;
    require_all(
        &architecture,
        &[
            "equal-or-stricter union",
            "Schema equality is not behavior equality",
            "must not delegate to Node",
            "previous native release",
            "never merged into `main` or `rust-build`",
        ],
    );
    let worker = std::fs::read_to_string(plan.join("WORKER_CHECKLIST.md"))?;
    require_all(
        &worker,
        &[
            "same fixture/input/config",
            "never use it to make a public row pass",
            "do not self-promote",
            "Immediate stop",
        ],
    );
    let index = std::fs::read_to_string(plan.join("WORKPACK_INDEX.md"))?;
    let indexed_rows = index
        .lines()
        .filter(|line| line.starts_with("| RM"))
        .count();
    assert_eq!(
        indexed_rows, 15,
        "workpack index must have exactly 15 RM rows"
    );
    let workpacks: Vec<_> = std::fs::read_dir(plan.join("workpacks"))?
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("rm"))
        .collect();
    assert_eq!(
        workpacks.len(),
        15,
        "must have exactly 15 RM workpack files"
    );
    for id in WORKPACK_IDS {
        let workpack = format!("{}-", id.to_ascii_lowercase());
        let matched: Vec<_> = workpacks
            .iter()
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(&workpack))
            .collect();
        assert_eq!(matched.len(), 1, "missing or duplicate workpack {id}");
        let entry = matched[0];
        let source = std::fs::read_to_string(entry.path())?;
        require_all(
            &source,
            &[
                "> Plan: rust-mjs-parity-retirement-plan",
                &format!("id: {id}"),
            ],
        );
        let index_marker = format!("| {id} |");
        assert_eq!(
            index
                .lines()
                .filter(|line| line.starts_with(&index_marker))
                .count(),
            1,
            "{id} must be indexed exactly once"
        );
    }
    Ok(())
}

#[test]
fn parity_retirement_plan_forbids_mjs_fallback_at_closure() -> TestResult {
    let root = workspace_root()?;
    let source = std::fs::read_to_string(root.join(
        "docs/plans/rust-mjs-parity-retirement-plan/workpacks/rm14-delete-not-merge-retirement.md",
    ))?;
    require_all(
        &source,
        &[
            "Delete-Not-Merge",
            "No executable MJS enforcement path remains",
            "without Node",
            "Do not merge the private overlay",
            "runtime MJS fallback",
        ],
    );
    Ok(())
}
