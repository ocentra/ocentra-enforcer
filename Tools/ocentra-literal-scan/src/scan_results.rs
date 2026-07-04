use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::risk::classify_literal;
use crate::{CliOptions, FileResult, Finding, RiskCategory};

pub(crate) fn classify_scan_results(
    mut results: Vec<FileResult>,
    opts: &CliOptions,
) -> (usize, Vec<Finding>, Vec<Finding>, BTreeMap<String, usize>) {
    let (literals_found, literal_locations) = collect_literal_locations(&results);
    let mut hard_findings = Vec::new();
    let mut literal_risks = Vec::new();
    let mut languages = BTreeMap::new();

    for result in &mut results {
        *languages.entry(result.language.clone()).or_insert(0) += 1;
        classify_result_candidates(
            result,
            opts,
            &literal_locations,
            &mut hard_findings,
            &mut literal_risks,
        );
        hard_findings.append(&mut result.findings);
    }

    hard_findings.sort_by_key(Finding::stable_key);
    literal_risks.sort_by_key(Finding::stable_key);
    (literals_found, hard_findings, literal_risks, languages)
}

fn collect_literal_locations(results: &[FileResult]) -> (usize, HashMap<String, BTreeSet<String>>) {
    let mut literal_locations: HashMap<String, BTreeSet<String>> = HashMap::new();
    let mut literals_found = 0usize;
    for result in results {
        literals_found += result.candidates.len();
        for candidate in &result.candidates {
            literal_locations
                .entry(candidate.text.clone())
                .or_default()
                .insert(result.file.clone());
        }
    }
    (literals_found, literal_locations)
}

fn classify_result_candidates(
    result: &mut FileResult,
    opts: &CliOptions,
    literal_locations: &HashMap<String, BTreeSet<String>>,
    hard_findings: &mut Vec<Finding>,
    literal_risks: &mut Vec<Finding>,
) {
    for candidate in result.candidates.drain(..) {
        let repeated_files = literal_locations
            .get(&candidate.text)
            .map(BTreeSet::len)
            .unwrap_or(0);
        let finding = classify_literal(
            &candidate,
            &result.file,
            &result.language,
            result.role,
            repeated_files,
            opts.fail_above,
        );
        if should_skip_low_import_finding(&finding, opts.include_low) {
            continue;
        }
        if finding.blocking {
            hard_findings.push(finding);
            continue;
        }
        if opts.include_low || finding.score >= opts.min_score {
            literal_risks.push(finding);
        }
    }
}

fn should_skip_low_import_finding(finding: &Finding, include_low: bool) -> bool {
    finding.category == RiskCategory::ImportSpecifier && !include_low
}
