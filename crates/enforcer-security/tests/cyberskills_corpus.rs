//! Pen-test-grade labeled corpus for the h11 SKILL.md frontmatter linter.
//! `_corpus/frontmatter.json` holds many full SKILL.md inputs labeled by
//! VENDOR behavior (`validate-skill.py` + the h11 mitre/nist extension):
//! `flag` = a violation the linter must catch, `clean` = a valid skill the
//! linter must accept. Proves detection (every structural violation +
//! malformed threat id flagged) AND no-false-positives (valid skills,
//! including alias subdomains and folded-scalar descriptions, stay clean).
//! Corpus reconciled against the vendor: mismatches were adjudicated to the
//! vendor verdict, not force-passed.

use std::path::{Path, PathBuf};

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_security::cyberskills::frontmatter_lint::SkillFrontmatterValidValidator;
use enforcer_validator::validator::{ValidationInput, Validator};

#[derive(serde::Deserialize)]
struct Case {
    name: String,
    input: String,
    expect: String,
    #[serde(default)]
    reason: String,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn corpus_frontmatter_lint() -> Result<(), Box<dyn std::error::Error>> {
    let path = manifest_dir().join("tests/fixtures/cyberskills/_corpus/frontmatter.json");
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read corpus {}: {e}", path.display()))?;
    let cases: Vec<Case> = serde_json::from_str(&raw)?;
    assert!(!cases.is_empty(), "empty frontmatter corpus");

    let validator = SkillFrontmatterValidValidator::new()?;
    let file: RelPath = "SKILL.md".parse()?;

    let mut mismatches = Vec::new();
    for case in &cases {
        let findings = validator
            .validate(ValidationInput {
                file: &file,
                source: &case.input,
                scope: ScanScope::Files,
            })
            .len();
        let flagged = findings > 0;
        let want_flag = match case.expect.as_str() {
            "flag" => true,
            "clean" => false,
            other => return Err(format!("case `{}`: bad expect `{other}`", case.name).into()),
        };
        if flagged != want_flag {
            mismatches.push(format!(
                "  [{}] expected {} but got {} findings ({}). reason: {}",
                case.name,
                case.expect,
                findings,
                if flagged { "flagged" } else { "clean" },
                case.reason
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "frontmatter.json: {} of {} cases mismatched:\n{}",
        mismatches.len(),
        cases.len(),
        mismatches.join("\n")
    );
    Ok(())
}

fn find_skill_md(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_skill_md(&path, out);
        } else if path.file_name().and_then(|n| n.to_str()) == Some("SKILL.md") {
            out.push(path);
        }
    }
}

/// The strongest parity proof: run the linter over EVERY one of the 817
/// vendored SKILL.md and assert it raises no STRUCTURAL false positive.
/// Those files are the canonical corpus `validate-skill.py` accepts, so any
/// structural finding (missing field, bad kebab, short description, wrong
/// domain, unknown subdomain, <2 tags) on a real skill is a parser or
/// allowlist bug — exactly the class the old fabricated subdomain list and
/// the folded-scalar parser gap produced (77 and 12 skills respectively).
/// The mitre/nist id extension is OURS (the vendor does not check ids), so
/// findings from it are excluded from this structural gate. Skipped
/// gracefully when the vendor dir is absent (L12 honesty protocol).
#[test]
fn real_corpus_has_no_structural_false_positives() -> Result<(), Box<dyn std::error::Error>> {
    let corpus_dir = manifest_dir().join("../../vendor/anthropic-cybersecurity-skills/skills");
    if !corpus_dir.is_dir() {
        return Ok(());
    }

    const STRUCTURAL_MARKERS: &[&str] = &[
        "missing required field",
        "not valid kebab-case",
        "name too long",
        "description too short",
        "domain must be",
        "unknown subdomain",
        "need at least 2 tags",
    ];

    let validator = SkillFrontmatterValidValidator::new()?;
    let file: RelPath = "SKILL.md".parse()?;

    let mut files = Vec::new();
    find_skill_md(&corpus_dir, &mut files);
    assert!(
        !files.is_empty(),
        "expected to find vendored SKILL.md files"
    );

    let mut offenders = Vec::new();
    for path in &files {
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: &source,
            scope: ScanScope::Files,
        });
        for finding in &findings {
            if STRUCTURAL_MARKERS
                .iter()
                .any(|m| finding.detail.contains(m))
            {
                offenders.push(format!("{}: {}", path.display(), finding.detail));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "structural false-positives on {} of {} real corpus skills (parser/allowlist bug):\n{}",
        offenders.len(),
        files.len(),
        offenders
            .iter()
            .take(25)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    Ok(())
}
