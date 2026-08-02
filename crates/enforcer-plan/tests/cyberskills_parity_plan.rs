//! Mechanical self-validation for the CyberSkills parity execution plan.
//!
//! The plan refines the broad h11/h12 umbrella into bounded workpacks.  This
//! test runs the live PLAN-* validators over every child workpack so the
//! execution contract cannot quietly decay into unchecked prose.

use std::path::{Path, PathBuf};

use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::ScanScope;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_plan::validator::{PlanFrontmatterValidator, PlanSkeletonValidator};
use enforcer_validator::validator::{ValidationInput, Validator};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const WORKPACK_IDS: [&str; 14] = [
    "CP00", "CP01", "CP02", "CP03", "CP04", "CP05", "CP06", "CP07", "CP08", "CP09", "CP10", "CP11",
    "CP12", "CP13",
];

fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "expected enforcer-plan below the workspace root".into())
}

fn rule_id(value: &str) -> Result<RuleId, Box<dyn std::error::Error>> {
    Ok(value.parse()?)
}

fn require_markers(source: &str, markers: &[&str], context: &str) {
    for marker in markers {
        assert!(
            source.contains(marker),
            "{context} missing required marker: {marker}"
        );
    }
}

fn workpack_row<'a>(index: &'a str, id: &str) -> Result<&'a str, std::io::Error> {
    let rows: Vec<&str> = index
        .lines()
        .filter(|line| line.split('|').nth(2).is_some_and(|cell| cell.trim() == id))
        .collect();
    if rows.len() != 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{id} must have exactly one routing row"),
        ));
    }
    Ok(rows[0])
}

#[test]
fn all_cyberskills_workpacks_pass_live_plan_validators() -> TestResult {
    let root = workspace_root()?;
    let workpacks_dir = root.join("docs/plans/cyberskills-parity-plan/workpacks");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&workpacks_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("md"))
        .collect();
    paths.sort();
    assert_eq!(paths.len(), 14, "CP00 through CP13 must all exist");

    let skeleton = PlanSkeletonValidator::new(rule_id("PLAN-SKELETON.1")?);
    let frontmatter = PlanFrontmatterValidator::new(rule_id("PLAN-FRONTMATTER.1")?);

    let mut failures = Vec::new();
    for path in paths {
        let source = std::fs::read_to_string(&path)?;
        let relative = path
            .strip_prefix(&root)?
            .to_string_lossy()
            .replace('\\', "/");
        let file: RelPath = relative.parse()?;
        let input = || ValidationInput {
            file: &file,
            source: ValidationSource::from_text(&source),
            scope: ScanScope::Files,
        };
        failures.extend(skeleton.validate(input()));
        failures.extend(frontmatter.validate(input()));
    }

    assert!(
        failures.is_empty(),
        "CyberSkills workpacks failed live PLAN-* validators: {failures:#?}"
    );
    Ok(())
}

#[test]
fn workpack_index_routes_every_workpack_exactly_once() -> TestResult {
    let root = workspace_root()?;
    let index =
        std::fs::read_to_string(root.join("docs/plans/cyberskills-parity-plan/WORKPACK_INDEX.md"))?;

    for id in WORKPACK_IDS {
        workpack_row(&index, id)?;
    }
    Ok(())
}

#[test]
fn cyberskills_plan_preserves_reuse_first_singletons_and_honest_unavailability() -> TestResult {
    let root = workspace_root()?;
    let plan_root = root.join("docs/plans/cyberskills-parity-plan");
    let read = |name: &str| std::fs::read_to_string(plan_root.join(name));

    let readme = read("README.md")?;
    let architecture = read("ARCHITECTURE.md")?;
    let index = read("WORKPACK_INDEX.md")?;
    let proof = read("TEST_PROOF_EXPECTATIONS.md")?;
    let state = read("PLAN_STATE.md")?;
    let agents = read("AGENTS.md")?;

    let unavailable = "df48fa4149dd25956e730443d3582693a3f825a8";
    for source in [&readme, &architecture, &proof, &state, &agents] {
        require_markers(
            source,
            &["sourceUnavailable", unavailable],
            "source identity contract",
        );
    }
    require_markers(
        &readme,
        &["817 tracked", "| 816 |", "never counted as covered"],
        "README corpus accounting",
    );

    for source in [&agents, &index] {
        require_markers(
            source,
            &["cyberskills-ledger-integrator", "tool-adapter-integrator"],
            "singleton ownership",
        );
    }
    require_markers(
        &architecture,
        &[
            "Universal UL02/UL03/UL04",
            "Universal UL13 graph/provider contract",
            "Universal UL07 deepens",
            "CP06 does not create or modify the generic runner",
        ],
        "cross-program architecture",
    );
    require_markers(workpack_row(&index, "CP02")?, &["UL02, UL03"], "CP02 row");
    require_markers(workpack_row(&index, "CP03")?, &["CP02, UL04"], "CP03 row");
    require_markers(
        workpack_row(&index, "CP06")?,
        &["never generic runner/registry/schema"],
        "CP06 row",
    );
    require_markers(
        workpack_row(&index, "CP12")?,
        &["CP03, CP08, UL13"],
        "CP12 row",
    );
    require_markers(
        &proof,
        &[
            "proof/cyberskills/cp13/closure.json",
            "independent clean-worktree reproduction",
        ],
        "CP13 proof contract",
    );

    for entry in std::fs::read_dir(plan_root.join("workpacks"))? {
        let path = entry?.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("md") {
            let source = std::fs::read_to_string(&path)?;
            require_markers(
                &source,
                &["> Plan: `cyberskills-parity-plan`"],
                &format!("{} capsule", path.display()),
            );
        }
    }
    Ok(())
}
