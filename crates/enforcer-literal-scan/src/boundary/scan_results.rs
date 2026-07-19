use std::collections::{BTreeMap, BTreeSet, HashMap};

use enforcer_domain::scan_types::{LiteralLanguageId, LiteralScanCount, LiteralStableHash};

use crate::risk::{classify_literal, ClassificationInput};
use crate::scan_types::FileResult;
use crate::stable_hash::stable_hash_key;
use crate::{CliOptions, Finding, RiskCategory};

pub(crate) fn classify_scan_results(
    results: Vec<FileResult>,
    opts: &CliOptions,
) -> (
    usize,
    Vec<Finding>,
    Vec<Finding>,
    BTreeMap<LiteralLanguageId, LiteralScanCount>,
) {
    let (literals_found, literal_locations) = collect_literal_locations(&results);
    let mut hard_findings = Vec::new();
    let mut literal_risks = Vec::new();
    let mut languages = BTreeMap::new();

    for mut result in results {
        classify_result_candidates(
            &mut result,
            opts,
            &literal_locations,
            &mut hard_findings,
            &mut literal_risks,
        );
        hard_findings.append(&mut result.findings);
        *languages.entry(result.language).or_default() += 1;
    }

    hard_findings.sort_by_key(Finding::stable_key);
    literal_risks.sort_by_key(Finding::stable_key);
    (literals_found, hard_findings, literal_risks, languages)
}

fn collect_literal_locations(
    results: &[FileResult],
) -> (
    usize,
    HashMap<LiteralStableHash, BTreeSet<LiteralStableHash>>,
) {
    let mut literal_locations: HashMap<LiteralStableHash, BTreeSet<LiteralStableHash>> =
        HashMap::new();
    let mut literals_found = 0usize;
    for result in results {
        literals_found += result.candidates.len();
        for candidate in &result.candidates {
            literal_locations
                .entry(stable_hash_key(candidate.text.as_str()))
                .or_default()
                .insert(stable_hash_key(result.file.as_str()));
        }
    }
    (literals_found, literal_locations)
}

fn classify_result_candidates(
    result: &mut FileResult,
    opts: &CliOptions,
    literal_locations: &HashMap<LiteralStableHash, BTreeSet<LiteralStableHash>>,
    hard_findings: &mut Vec<Finding>,
    literal_risks: &mut Vec<Finding>,
) {
    for candidate in result.candidates.drain(..) {
        let repeated_files = literal_locations
            .get(&stable_hash_key(candidate.text.as_str()))
            .map(BTreeSet::len)
            .unwrap_or(0);
        let finding = classify_literal(ClassificationInput {
            candidate: &candidate,
            file: &result.file,
            language: &result.language,
            role: result.role,
            repeated_files,
            fail_above: opts.fail_above,
        });
        if should_skip_low_import_finding(&finding, opts.include_low.is_enabled()) {
            continue;
        }
        if finding.blocking.is_blocking() {
            hard_findings.push(finding);
            continue;
        }
        if opts.include_low.is_enabled() || finding.score >= opts.min_score {
            literal_risks.push(finding);
        }
    }
}

fn should_skip_low_import_finding(finding: &Finding, include_low: bool) -> bool {
    finding.category == RiskCategory::ImportSpecifier && !include_low
}
