//! Native, bounded signal analysis for the test-doctrine MCP tool.
//!
//! This is intentionally a project-posture report, not a test runner and not
//! a certification. It uses the same evidence classes as the frozen scanner:
//! file names, manifests, bounded test-file content, and CI step text.

use std::collections::BTreeMap;
use std::path::Path;

use enforcer_domain::paths::RepoRoot;
use enforcer_domain::test_doctrine_types::{
    TestDoctrineBlockingState, TestDoctrineCategory as Category, TestDoctrineCiConfigFile,
    TestDoctrineCiEvidence, TestDoctrineCiGap, TestDoctrineCiState, TestDoctrineCount,
    TestDoctrineDetection, TestDoctrineEvidenceState, TestDoctrineMissing, TestDoctrineNature,
    TestDoctrineReport, TestDoctrineSummary, TestDoctrineText, TestDoctrineTier as Tier,
};

use crate::walk::{self, IgnoreRules};

const CONTENT_SCAN_FILE_CAP: usize = 300;
const CAVEAT: &str = "Heuristic, signal-based (file names, config files, dependency manifests, CI step text); not a certification. Evidence should be opened and judged, not trusted at face value; absence of a signal does not always mean absence of the practice.";

/// Analyze a repository's test posture using only native Rust filesystem and
/// text processing. Discovery honours the scanner's generated/VCS ignores.
pub fn analyze(root: &RepoRoot) -> Result<TestDoctrineReport, std::io::Error> {
    let root_path = Path::new(root.as_str());
    let rel_paths = walk::walk(root_path, &IgnoreRules::default())?;
    let paths: Vec<String> = rel_paths
        .iter()
        .map(|path| path.as_str().to_owned())
        .collect();
    let manifest_text = collect_manifest_text(root_path, &paths);
    let nature = detect_nature(&paths, &manifest_text);
    let ci = analyze_ci(root_path, &paths, &manifest_text);

    let mut detected = BTreeMap::new();
    let mut missing = Vec::new();
    let mut ci_gaps = Vec::new();
    for category in categories() {
        let (present, evidence) = detect_category(category, root_path, &paths, &manifest_text);
        let (relevant, tier) = relevance(category, &nature);
        let ci_state = ci.states.get(&category).cloned().unwrap_or_else(empty_ci);
        let detection = TestDoctrineDetection::new(text(label(category)))
            .with_evidence(state(present), evidence.into_iter().map(text).collect())
            .with_relevance(state(relevant))
            .with_ci(ci_state.clone(), Some(ci_state.clone()));
        if relevant && !present {
            missing.push(TestDoctrineMissing::new(
                category,
                text(label(category)),
                tier,
                text(missing_reason(category, &nature)),
            ));
        } else if relevant && present && !ci_state.is_blocking() {
            ci_gaps.push(TestDoctrineCiGap::new(
                category,
                text(label(category)),
                text(ci_gap_reason(&ci_state)),
                ci_state.evidence().to_vec(),
            ));
        }
        detected.insert(category, detection);
    }
    missing.sort_by_key(|entry| match entry.tier() {
        Tier::Core => 0,
        Tier::Suggested => 1,
        Tier::Optional => 2,
    });
    let categories_relevant = detected.values().filter(|item| item.is_relevant()).count();
    let categories_present = detected
        .values()
        .filter(|item| item.is_relevant() && item.is_present())
        .count();
    let core_missing = missing
        .iter()
        .filter(|item| item.tier() == Tier::Core)
        .count();
    let summary = TestDoctrineSummary::new(
        count(categories_relevant),
        count(categories_present),
        count(missing.len()),
        count(core_missing),
        count(ci_gaps.len()),
    );
    Ok(
        TestDoctrineReport::new(text(root.as_str()), text(CAVEAT), nature)
            .with_ci_files(ci.files, state(false))
            .with_results(detected, missing, ci_gaps, summary),
    )
}

fn text(value: impl Into<String>) -> TestDoctrineText {
    TestDoctrineText::from_owned(value.into())
}
fn count(value: usize) -> TestDoctrineCount {
    TestDoctrineCount::from_usize(value)
}
fn state(value: bool) -> TestDoctrineEvidenceState {
    TestDoctrineEvidenceState::from_bool(value)
}

fn categories() -> [Category; 16] {
    [
        Category::Unit,
        Category::Integration,
        Category::E2e,
        Category::Contract,
        Category::Mutation,
        Category::PropertyFuzzing,
        Category::Security,
        Category::Snapshot,
        Category::LoadPerformance,
        Category::CoverageTooling,
        Category::ConcurrencyRaceTests,
        Category::IdempotencyReplayTests,
        Category::RollbackCompensationTests,
        Category::TimeClockTests,
        Category::EconomicInvariantTests,
        Category::KillSwitchTests,
    ]
}

fn label(category: Category) -> &'static str {
    match category {
        Category::Unit => "Unit tests",
        Category::Integration => "Integration tests",
        Category::E2e => "End-to-end (Playwright/Cypress)",
        Category::Contract => "Contract tests",
        Category::Mutation => "Mutation testing",
        Category::PropertyFuzzing => "Property-based / fuzz testing",
        Category::Security => "Security test tooling",
        Category::Snapshot => "Snapshot testing",
        Category::LoadPerformance => "Load/performance testing",
        Category::CoverageTooling => "Coverage tooling",
        Category::ConcurrencyRaceTests => "Concurrency / race-condition tests",
        Category::IdempotencyReplayTests => "Idempotency / replay tests",
        Category::RollbackCompensationTests => "Rollback / compensation tests",
        Category::TimeClockTests => "Time / clock-manipulation tests",
        Category::EconomicInvariantTests => "Economic / balance-invariant tests",
        Category::KillSwitchTests => "Kill-switch / circuit-breaker tests",
    }
}

fn is_manifest(path: &str) -> bool {
    let file = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    matches!(
        file.as_str(),
        "package.json" | "pyproject.toml" | "cargo.toml" | "pipfile"
    ) || file.starts_with("requirements") && file.ends_with(".txt")
}
fn read(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap_or_default()
}
fn lower(value: &str) -> String {
    value.to_ascii_lowercase()
}
fn any_contains(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn collect_manifest_text(root: &Path, paths: &[String]) -> String {
    paths
        .iter()
        .filter(|path| is_manifest(path))
        .map(|path| read(root, path))
        .collect::<Vec<_>>()
        .join("\n")
}

fn detect_nature(paths: &[String], manifest: &str) -> TestDoctrineNature {
    let mut languages = BTreeMap::new();
    for (name, suffixes) in [
        ("python", &[".py"][..]),
        ("typescript", &[".ts", ".tsx"][..]),
        ("javascript", &[".js", ".jsx"][..]),
        ("rust", &[".rs"][..]),
        ("go", &[".go"][..]),
    ] {
        let count = paths
            .iter()
            .filter(|path| suffixes.iter().any(|suffix| path.ends_with(suffix)))
            .count();
        if count > 0 {
            languages.insert(name.to_owned(), count);
        }
    }
    let manifest_lower = lower(manifest);
    let has_open_api_spec = paths.iter().any(|path| {
        let path = lower(path);
        path.ends_with("openapi.json")
            || path.ends_with("openapi.yaml")
            || path.ends_with("openapi.yml")
            || path.ends_with("swagger.json")
            || path.ends_with("swagger.yaml")
            || path.ends_with("swagger.yml")
    });
    let money_critical_files: Vec<String> = paths
        .iter()
        .filter(|path| {
            any_contains(
                &lower(path),
                &[
                    "billing",
                    "payment",
                    "invoice",
                    "stripe",
                    "wallet",
                    "credit",
                    "balance",
                    "pricing",
                    "checkout",
                    "subscription",
                ],
            )
        })
        .take(10)
        .cloned()
        .collect();
    let multi_service_client_files: Vec<String> = paths
        .iter()
        .filter(|path| {
            let p = lower(path);
            p.rsplit('/').next().is_some_and(|file| {
                file.contains("client.")
                    && [".py", ".ts", ".tsx", ".js", ".jsx"]
                        .iter()
                        .any(|suffix| file.ends_with(suffix))
            })
        })
        .take(10)
        .cloned()
        .collect();
    TestDoctrineNature::new(
        languages
            .into_iter()
            .map(|(language, total)| (text(language), count(total)))
            .collect(),
    )
    .with_web_api(state(
        has_open_api_spec
            || any_contains(
                &manifest_lower,
                &[
                    "fastapi",
                    "flask",
                    "django",
                    "express",
                    "@nestjs",
                    "koa",
                    "actix-web",
                    "axum",
                ],
            ),
    ))
    .with_open_api(state(has_open_api_spec))
    .with_frontend(state(any_contains(
        &manifest_lower,
        &["\"react\"", "\"vue\"", "\"@angular/core\"", "\"svelte\""],
    )))
    .with_async_workers(state(
        any_contains(
            &manifest_lower,
            &[
                "boto3",
                "celery",
                "bullmq",
                "kafka-python",
                "pika",
                "aiokafka",
            ],
        ) || paths.iter().any(|path| lower(path).contains("/worker")),
    ))
    .with_money_surface(
        state(!money_critical_files.is_empty()),
        money_critical_files.into_iter().map(text).collect(),
    )
    .with_service_boundary(
        state(!multi_service_client_files.is_empty()),
        multi_service_client_files.into_iter().map(text).collect(),
    )
}

fn is_test_file(path: &str) -> bool {
    let p = lower(path);
    p.contains(".test.")
        || p.contains(".spec.")
        || p.rsplit('/')
            .next()
            .is_some_and(|file| file.starts_with("test_") || file.ends_with("_test.py"))
}
fn filename_hit(category: Category, path: &str) -> bool {
    let p = lower(path);
    match category {
        Category::Unit => {
            is_test_file(&p)
                || p.starts_with("test/")
                || p.starts_with("tests/")
                || p.contains("/test/")
                || p.contains("/tests/")
        }
        Category::Integration => {
            p.starts_with("test/integration/")
                || p.starts_with("tests/integration/")
                || p.contains("/test/integration/")
                || p.contains("/tests/integration/")
                || p.contains("integration-tests/")
                || p.contains("integration_tests/")
                || p.contains(".integration.test")
                || p.contains(".integration.spec")
        }
        Category::E2e => {
            p.contains("/e2e/")
                || p.contains("/end-to-end/")
                || p.ends_with("playwright.config.ts")
                || p.ends_with("playwright.config.js")
                || p.ends_with("cypress.config.ts")
                || p.ends_with("cypress.config.js")
                || p.contains("/cypress/")
        }
        Category::Contract => {
            p.contains("/pact") || p.contains("contract") || p.contains("schemathesis")
        }
        Category::Mutation => {
            p.contains("stryker.conf")
                || p.ends_with("mutmut.ini")
                || p.ends_with("cosmic-ray.toml")
                || p.ends_with(".cargo-mutants.toml")
        }
        Category::PropertyFuzzing => p.contains("/.hypothesis/"),
        Category::Security => {
            p.ends_with(".semgrep.yml")
                || p.ends_with(".semgrep.yaml")
                || p.contains("/.github/codeql")
                || p.ends_with("gitleaks.toml")
                || p.contains("/.zap/")
                || p.ends_with("/.bandit")
        }
        Category::Snapshot => p.contains("/__snapshots__/") || p.ends_with(".snap"),
        Category::LoadPerformance => {
            p.contains("/k6/")
                || p.ends_with("artillery.yml")
                || p.ends_with("artillery.yaml")
                || p.ends_with("artillery.json")
                || p.ends_with("locustfile.py")
                || p.contains(".gatling.")
        }
        Category::CoverageTooling => {
            p.contains(".nycrc") || p.contains("/c8.config") || p.ends_with(".coveragerc")
        }
        Category::ConcurrencyRaceTests => {
            p.contains("concurrency")
                || p.contains("concurrent")
                || p.contains("race-condition")
                || p.contains("race_condition")
        }
        Category::IdempotencyReplayTests => {
            p.contains("idempotency") || p.contains("idempotent") || p.contains("replay")
        }
        Category::RollbackCompensationTests => p.contains("rollback") || p.contains("compensat"),
        Category::TimeClockTests => false,
        Category::EconomicInvariantTests => false,
        Category::KillSwitchTests => {
            p.contains("kill-switch")
                || p.contains("kill_switch")
                || p.contains("circuit-breaker")
                || p.contains("circuit_breaker")
                || p.contains("emergency-stop")
                || p.contains("emergency_disable")
        }
    }
}

fn manifest_hit(category: Category, manifest: &str) -> bool {
    let m = lower(manifest);
    match category {
        Category::E2e => any_contains(
            &m,
            &[
                "@playwright/test",
                "\"cypress\"",
                "puppeteer",
                "selenium-webdriver",
            ],
        ),
        Category::Contract => any_contains(
            &m,
            &[
                "@pact-foundation/pact",
                "pact-python",
                "schemathesis",
                "openapi-diff",
            ],
        ),
        Category::Mutation => any_contains(
            &m,
            &[
                "@stryker-mutator/core",
                "mutmut",
                "cargo-mutants",
                "cosmic-ray",
            ],
        ),
        Category::PropertyFuzzing => any_contains(
            &m,
            &[
                "fast-check",
                "hypothesis",
                "proptest",
                "quickcheck",
                "schemathesis",
                "atheris",
            ],
        ),
        Category::Security => any_contains(&m, &["bandit", "semgrep", "pip-audit", "safety"]),
        Category::LoadPerformance => any_contains(&m, &["\"k6\"", "artillery", "locust"]),
        Category::CoverageTooling => {
            any_contains(&m, &["pytest-cov", "\"c8\"", "\"nyc\"", "coverage[toml]"])
        }
        Category::TimeClockTests => any_contains(&m, &["freezegun", "time-machine"]),
        _ => false,
    }
}
fn content_needles(category: Category) -> &'static [&'static str] {
    match category {
        Category::ConcurrencyRaceTests => &[
            "asyncio.gather",
            "promise.all",
            "concurrent.futures",
            "threadpoolexecutor",
            "race condition",
            "race-condition",
            "retry storm",
        ],
        Category::IdempotencyReplayTests => &[
            "idempoten",
            "replay",
            "duplicate-request",
            "duplicate request",
        ],
        Category::RollbackCompensationTests => &["rollback", "compensat"],
        Category::TimeClockTests => &[
            "freeze_time",
            "usefaketimers",
            "travel_to",
            "clock skew",
            "mock datetime",
            "mock clock",
        ],
        Category::EconomicInvariantTests => &[
            "invariant",
            "balance unchanged",
            "balance preserved",
            "balance conserv",
            "double-spend",
            "double charge",
            "double-charge",
            "double-book",
        ],
        Category::KillSwitchTests => &[
            "kill-switch",
            "kill switch",
            "circuit-breaker",
            "circuit breaker",
            "emergency disable",
            "emergency stop",
            "emergency halt",
        ],
        _ => &[],
    }
}
fn detect_category(
    category: Category,
    root: &Path,
    paths: &[String],
    manifest: &str,
) -> (bool, Vec<String>) {
    let mut evidence: Vec<String> = paths
        .iter()
        .filter(|path| filename_hit(category, path))
        .take(5)
        .cloned()
        .collect();
    if manifest_hit(category, manifest) {
        evidence.push("manifest: dependency/config signal".to_owned());
    }
    if !content_needles(category).is_empty() {
        for path in paths
            .iter()
            .filter(|path| is_test_file(path))
            .take(CONTENT_SCAN_FILE_CAP)
        {
            let text = lower(&read(root, path));
            if let Some(needle) = content_needles(category)
                .iter()
                .find(|needle| text.contains(**needle))
            {
                evidence.push(format!("{path} (matched: {needle})"));
                if evidence.len() >= 5 {
                    break;
                }
            }
        }
    }
    (!evidence.is_empty(), evidence)
}

fn relevance(category: Category, nature: &TestDoctrineNature) -> (bool, Tier) {
    let critical_tier = if nature.has_money_critical_surface() {
        Tier::Core
    } else {
        Tier::Suggested
    };
    match category {
        Category::Unit | Category::Security | Category::CoverageTooling => (true, Tier::Core),
        Category::Mutation | Category::Snapshot => (true, Tier::Optional),
        Category::Integration => (
            nature.is_web_api() || nature.has_async_workers(),
            Tier::Core,
        ),
        Category::E2e => (nature.has_frontend_ui(), Tier::Core),
        Category::Contract => (
            nature.is_web_api() || nature.has_multi_service_boundary(),
            Tier::Core,
        ),
        Category::PropertyFuzzing => (true, Tier::Suggested),
        Category::LoadPerformance => (
            nature.is_web_api() || nature.has_async_workers(),
            Tier::Suggested,
        ),
        Category::ConcurrencyRaceTests => (
            nature.is_web_api() || nature.has_async_workers(),
            critical_tier,
        ),
        Category::IdempotencyReplayTests => (
            nature.is_web_api()
                || nature.has_multi_service_boundary()
                || nature.has_money_critical_surface(),
            critical_tier,
        ),
        Category::RollbackCompensationTests => (
            nature.has_money_critical_surface() || nature.has_async_workers(),
            critical_tier,
        ),
        Category::TimeClockTests => (
            true,
            if nature.has_money_critical_surface() {
                Tier::Suggested
            } else {
                Tier::Optional
            },
        ),
        Category::EconomicInvariantTests => (nature.has_money_critical_surface(), Tier::Core),
        Category::KillSwitchTests => (nature.has_money_critical_surface(), Tier::Suggested),
    }
}
fn missing_reason(category: Category, nature: &TestDoctrineNature) -> String {
    let money_note = if nature.has_money_critical_surface() {
        format!(
            " Money/billing-looking files: {}.",
            nature
                .money_critical_files()
                .iter()
                .take(3)
                .map(|value| value.as_str().to_owned())
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        String::new()
    };
    match category {
        Category::Unit => "No unit-test signal was found; isolated behavior is unproven.".to_owned(),
        Category::Security => "Baseline secret-scanning and static-analysis tooling protects against common incidents.".to_owned(),
        Category::CoverageTooling => "No coverage measurement tool found; exercised code cannot be measured.".to_owned(),
        Category::Integration => format!("Project looks like a web API or async worker; integration tests verify the actual request/consumer lifecycle.{money_note}"),
        Category::E2e => "A frontend UI was detected; no end-to-end suite means UI regressions are caught only by hand.".to_owned(),
        Category::Contract => "A public API or service boundary was detected; without contract tests either side can silently break the other.".to_owned(),
        Category::PropertyFuzzing => "No property-based or API-fuzz tooling found; generated edge cases are untested.".to_owned(),
        Category::LoadPerformance => "No load/performance tooling found; capacity and degradation behavior are untested.".to_owned(),
        Category::ConcurrencyRaceTests => format!("No tests exercising concurrent requests were found; race conditions only appear under real concurrency.{money_note}"),
        Category::IdempotencyReplayTests => format!("No tests asserting repeated requests do not repeat effects were found.{money_note}"),
        Category::RollbackCompensationTests => "No rollback/compensation tests were found; partial-failure recovery is unproven.".to_owned(),
        Category::TimeClockTests => "No tests manipulate time/clock; expiry and scheduling boundaries are untested.".to_owned(),
        Category::EconomicInvariantTests => format!("Money-critical files were detected but no explicit balance/invariant tests were found.{money_note}"),
        Category::KillSwitchTests => "Money-critical files were detected but no kill-switch/circuit-breaker test was found.".to_owned(),
        Category::Mutation | Category::Snapshot => "Not detected.".to_owned(),
    }
}

struct CiAnalysis {
    files: Vec<TestDoctrineCiConfigFile>,
    states: BTreeMap<Category, TestDoctrineCiState>,
}
fn is_ci(path: &str) -> bool {
    let p = lower(path);
    ((p.starts_with(".github/workflows/") || p.contains("/.github/workflows/"))
        && (p.ends_with(".yml") || p.ends_with(".yaml")))
        || p.ends_with(".gitlab-ci.yml")
        || p.ends_with("azure-pipelines.yml")
        || p.ends_with("azure-pipelines.yaml")
        || p.ends_with("jenkinsfile")
        || p.ends_with(".circleci/config.yml")
        || p.ends_with(".circleci/config.yaml")
}
fn strip_comments(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .map(|line| line.split(" #").next().unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}
fn step_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if (trimmed.starts_with("- name:")
            || trimmed.starts_with("- run:")
            || trimmed.starts_with("- uses:"))
            && !current.is_empty()
        {
            blocks.push(current.join("\n"));
            current.clear();
        }
        current.push(line);
    }
    if !current.is_empty() {
        blocks.push(current.join("\n"));
    }
    blocks
}
fn ci_patterns(category: Category) -> &'static [&'static str] {
    match category {
        Category::Unit => &["pytest", "npm test", "npm run test", "vitest", "cargo test"],
        Category::Integration => &["pytest"],
        Category::E2e => &["playwright test", "cypress run"],
        Category::Contract => &[
            "pact-broker",
            "pact verify",
            "schemathesis run",
            "prism mock",
        ],
        Category::Mutation => &["stryker run", "mutmut run", "cargo mutants"],
        Category::PropertyFuzzing => &["schemathesis run", "hypothesis"],
        Category::Security => &[
            "bandit",
            "semgrep",
            "gitleaks",
            "codeql",
            "npm audit",
            "uv audit",
            "trivy",
        ],
        Category::LoadPerformance => &["k6 run", "artillery run", "locust"],
        Category::CoverageTooling => &["--cov", "coverage run", "codecov"],
        _ => &[],
    }
}
fn empty_ci() -> TestDoctrineCiState {
    TestDoctrineCiState::new(
        state(false),
        TestDoctrineBlockingState::NotInvoked,
        Vec::new(),
    )
}
fn analyze_ci(root: &Path, paths: &[String], manifest: &str) -> CiAnalysis {
    let ci_paths: Vec<&String> = paths.iter().filter(|path| is_ci(path)).collect();
    let files = ci_paths
        .iter()
        .map(|path| TestDoctrineCiConfigFile::new(text((*path).clone()), state(true)))
        .collect();
    let texts: Vec<String> = ci_paths.iter().map(|path| read(root, path)).collect();
    let all_text = texts.join("\n");
    let blocks: Vec<String> = texts.iter().flat_map(|text| step_blocks(text)).collect();
    let mut states = BTreeMap::new();
    for category in categories() {
        let mut evidence = Vec::new();
        let patterns = ci_patterns(category);
        for block in &blocks {
            let clean = lower(&strip_comments(block));
            if !patterns.is_empty() && any_contains(&clean, patterns) {
                if category == Category::Integration
                    && !any_contains(
                        &lower(&all_text),
                        &["postgres", "redis", "mysql", "rabbitmq", "mongo"],
                    )
                {
                    continue;
                }
                let blocking = !any_contains(
                    &clean,
                    &["continue-on-error: true", "|| true", "allow_failure: true"],
                );
                evidence.push(TestDoctrineCiEvidence::new(
                    text(
                        block
                            .trim()
                            .lines()
                            .next()
                            .unwrap_or_default()
                            .trim()
                            .to_owned(),
                    ),
                    state(blocking),
                ));
            }
        }
        if category == Category::CoverageTooling
            && evidence.is_empty()
            && any_contains(
                &lower(manifest),
                &["--cov-fail-under", "fail_under =", "coveragethreshold"],
            )
        {
            evidence.push(TestDoctrineCiEvidence::new(
                text("coverage threshold enforced via test-runner config"),
                state(true),
            ));
        }
        evidence.sort_by_key(|item| !item.is_blocking());
        evidence.truncate(3);
        let blocking = if evidence.is_empty() {
            None
        } else {
            Some(evidence.iter().any(TestDoctrineCiEvidence::is_blocking))
        };
        states.insert(
            category,
            TestDoctrineCiState::new(
                state(!evidence.is_empty()),
                TestDoctrineBlockingState::from_evidence(
                    !evidence.is_empty(),
                    blocking.unwrap_or(false),
                ),
                evidence,
            ),
        );
    }
    CiAnalysis { files, states }
}
fn ci_gap_reason(ci: &TestDoctrineCiState) -> String {
    if !ci.is_wired() {
        "Detected locally but never invoked anywhere in committed CI; nothing forces it to run before merge.".to_owned()
    } else {
        "Runs in CI but every matching step is non-blocking (continue-on-error/allow_failure); a failure is not gated.".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{analyze, filename_hit, is_ci};
    use enforcer_domain::paths::RepoRoot;
    use enforcer_domain::test_doctrine_types::TestDoctrineCategory as Category;

    #[test]
    fn recognizes_root_relative_test_and_workflow_paths() {
        assert!(filename_hit(Category::Unit, "tests/helpers.rs"));
        assert!(filename_hit(
            Category::Integration,
            "tests/integration/http.rs"
        ));
        assert!(is_ci(".github/workflows/ci.yml"));
    }

    #[test]
    fn reports_nature_missing_categories_and_non_blocking_ci(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join(".github/workflows"))?;
        std::fs::create_dir_all(temp.path().join("tests"))?;
        std::fs::write(
            temp.path().join("package.json"),
            r#"{ "dependencies": { "express": "1" } }"#,
        )?;
        std::fs::write(
            temp.path().join("tests/unit.test.ts"),
            "it('works', () => {});",
        )?;
        std::fs::write(
            temp.path().join(".github/workflows/ci.yml"),
            "- run: npm test\n  continue-on-error: true\n",
        )?;
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let report = analyze(&root)?;
        assert!(report.nature().is_web_api());
        assert!(report
            .detection(Category::Unit)
            .is_some_and(|item| item.is_present()));
        assert!(report
            .missing()
            .iter()
            .any(|item| item.category() == Category::Security));
        assert!(report
            .ci_gaps()
            .iter()
            .any(|item| item.category() == Category::Unit));
        Ok(())
    }
}
