//! Native advisory ARCH-1.16 UI/logic coupling analysis.
//!
//! This deliberately mirrors the frozen signal scanner rather than claiming
//! AST certainty: presentation-path classification + import binding analysis
//! produces evidence for review and never changes scan/CI gate behavior.

use std::collections::BTreeSet;
use std::path::Path;

use enforcer_domain::paths::RepoRoot;
use enforcer_domain::ui_logic_coupling_types::{
    UiLogicCount as Count, UiLogicCouplingReport, UiLogicCouplingReportInput,
    UiLogicEvidenceState as State, UiLogicFinding, UiLogicFindingInput, UiLogicFindingKind as Kind,
    UiLogicFindingSeverity as Severity, UiLogicRule, UiLogicRuleInput, UiLogicSummary,
    UiLogicText as Text,
};

use crate::walk::{self, IgnoreRules};

const CAVEAT: &str = "Mechanical, signal-based (import paths + naming conventions) — not an AST parser. Every finding is evidence for human/AI review, not a certified defect. Run a second pass before treating any 'hard' finding as confirmed.";

/// Analyze one repository using the native ignored-directory walk.
pub fn analyze(root: &RepoRoot) -> Result<UiLogicCouplingReport, std::io::Error> {
    let root_path = Path::new(root.as_str());
    let paths = walk::walk(root_path, &IgnoreRules::default())?;
    let mut findings = Vec::new();
    for path in paths {
        let rel = path.as_str();
        if !is_presentation_file(rel) {
            continue;
        }
        let text = std::fs::read_to_string(root_path.join(rel)).unwrap_or_default();
        findings.extend(scan_file(rel, &text).map_err(invalid_boundary)?);
    }
    let hard: Vec<UiLogicFinding> = findings
        .iter()
        .filter(|finding| finding.is_hard())
        .cloned()
        .collect();
    let info: Vec<UiLogicFinding> = findings
        .iter()
        .filter(|finding| !finding.is_hard())
        .cloned()
        .collect();
    let hard_files: BTreeSet<String> = hard
        .iter()
        .map(|finding| finding.file().as_str().to_owned())
        .collect();
    let summary = UiLogicSummary::new(
        Count::try_from_len(findings.len()).map_err(invalid_boundary)?,
        Count::try_from_len(hard.len()).map_err(invalid_boundary)?,
        Count::try_from_len(info.len()).map_err(invalid_boundary)?,
        Count::try_from_len(hard_files.len()).map_err(invalid_boundary)?,
    );
    let rule = UiLogicRule::from_input(UiLogicRuleInput::new(
        Text::try_new("ARCH-1.16".to_owned()).map_err(invalid_boundary)?,
        Text::try_new("Presentation/UI cannot call business logic directly".to_owned()).map_err(invalid_boundary)?,
        Text::try_new("rules/common/architecture.md#covered-rules".to_owned()).map_err(invalid_boundary)?,
        Text::try_new("Humble Object pattern / UI half of Hexagonal (Ports-and-Adapters) architecture / the boundary unidirectional-data-flow (Flux/Redux/Elm) architectures enforce".to_owned()).map_err(invalid_boundary)?,
        Text::try_new("Lets a UI shell be replaced (web/mobile/desktop) without touching business logic, lets business logic be tested without rendering anything, and gives the boundary something to contract-test instead of testing everything through the UI.".to_owned()).map_err(invalid_boundary)?,
    ));
    let input = UiLogicCouplingReportInput::new(
        Text::try_new(root.as_str().to_owned()).map_err(invalid_boundary)?,
        rule,
        Text::try_new(CAVEAT.to_owned()).map_err(invalid_boundary)?,
    )
    .with_findings(findings)
    .with_summary(summary)
    .with_hard(hard)
    .with_info(info);
    Ok(UiLogicCouplingReport::from_input(input))
}

fn invalid_boundary(error: enforcer_domain::boundary::decode_error::DecodeError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error)
}

fn lower(value: &str) -> String {
    value.to_ascii_lowercase()
}
fn is_presentation_file(path: &str) -> bool {
    let lower = lower(path);
    let extension = [".ts", ".tsx", ".js", ".jsx", ".vue"]
        .iter()
        .any(|suffix| lower.ends_with(suffix));
    let tested = lower.contains(".test.") || lower.contains(".spec.");
    let hook = lower.contains("/hooks/")
        || lower.rsplit('/').next().is_some_and(|name| {
            name.starts_with("use")
                && [".ts", ".tsx", ".js", ".jsx"]
                    .iter()
                    .any(|suffix| name.ends_with(suffix))
        });
    extension
        && !tested
        && !hook
        && lower.split('/').any(|segment| {
            matches!(
                segment,
                "pages" | "components" | "views" | "screens" | "containers"
            )
        })
}

#[derive(Debug)]
struct Import {
    names: Vec<String>,
    source: String,
}

fn extract_imports(text: &str) -> Vec<Import> {
    text.lines().filter_map(parse_import).collect()
}
fn parse_import(line: &str) -> Option<Import> {
    let trimmed = line.trim();
    if !trimmed.starts_with("import ") || !trimmed.contains(" from ") {
        return None;
    }
    let (left, source_part) = trimmed.split_once(" from ")?;
    let source = source_part
        .trim()
        .trim_end_matches(';')
        .trim()
        .trim_matches(['\'', '"'])
        .to_owned();
    if source.is_empty() {
        return None;
    }
    let binding = left
        .trim_start_matches("import ")
        .trim_start_matches("type ")
        .trim();
    let names = if binding.starts_with('{') && binding.ends_with('}') {
        binding
            .trim_matches(['{', '}'])
            .split(',')
            .map(|entry| {
                entry
                    .trim()
                    .split_once(" as ")
                    .map_or_else(|| entry.trim(), |(name, _)| name.trim())
                    .to_owned()
            })
            .filter(|name| !name.is_empty())
            .collect()
    } else if binding
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        vec![binding.to_owned()]
    } else {
        Vec::new()
    };
    (!names.is_empty()).then_some(Import { names, source })
}
fn is_business_source(source: &str) -> bool {
    let source = lower(source);
    source.contains("/lib/api")
        || source.contains("/services/")
        || source.contains("/api/client")
        || source.contains("api-client")
}
fn is_event_source(source: &str) -> bool {
    let source = lower(source);
    source.contains("/lib/ws") || source.contains("/lib/socket") || source.contains("/realtime")
}
fn has_data_fetch(text: &str) -> bool {
    [
        "usequery(",
        "usemutation(",
        "useswr(",
        "useinfinitequery(",
        "createquery(",
    ]
    .iter()
    .any(|needle| lower(text).contains(needle))
}
fn binding_is_called(name: &str, text: &str) -> bool {
    let bytes = text.as_bytes();
    let needle = format!("{name}.");
    let mut offset = 0;
    while let Some(index) = text[offset..].find(&needle) {
        let start = offset + index + needle.len();
        let rest = &text[start..];
        let method = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .count();
        if method > 0 && rest[method..].trim_start().starts_with('(') {
            return true;
        }
        offset = start;
    }
    let _ = bytes;
    false
}
fn scan_file(
    path: &str,
    text: &str,
) -> Result<Vec<UiLogicFinding>, enforcer_domain::boundary::decode_error::DecodeError> {
    let data_fetch = State::from_bool(has_data_fetch(text));
    let mut findings = Vec::new();
    for import in extract_imports(text) {
        if is_business_source(&import.source) {
            for name in &import.names {
                if binding_is_called(name, text) {
                    // Frozen intent treats an Error-suffixed binding used only for narrowing as informational.
                    let severity = if name.ends_with("Error") {
                        Severity::Info
                    } else {
                        Severity::Hard
                    };
                    let input = UiLogicFindingInput::new(
                        Text::try_new(path.to_owned())?,
                        Text::try_new(import.source.clone())?,
                        Text::try_new(name.clone())?,
                    )
                    .with_kind(Kind::BusinessLogicImport)
                    .with_severity(severity)
                    .with_data_fetch(data_fetch);
                    findings.push(UiLogicFinding::from_input(input));
                }
            }
        }
        if is_event_source(&import.source) {
            let input = UiLogicFindingInput::new(
                Text::try_new(path.to_owned())?,
                Text::try_new(import.source)?,
                Text::try_new(import.names.join(", "))?,
            )
            .with_kind(Kind::EventSourceImport)
            .with_severity(Severity::Hard)
            .with_data_fetch(data_fetch);
            findings.push(UiLogicFinding::from_input(input));
        }
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::analyze;
    use enforcer_domain::paths::RepoRoot;

    fn report_for(files: &[(&str, &str)]) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        for (path, contents) in files {
            let full = temp.path().join(path);
            std::fs::create_dir_all(full.parent().ok_or("fixture path missing parent")?)?;
            std::fs::write(full, contents)?;
        }
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        Ok(serde_json::to_value(analyze(&root)?)?)
    }
    #[test]
    fn presentation_direct_api_call_is_hard_and_marks_data_fetch(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let report = report_for(&[(
            "src/components/Orders.tsx",
            "import { api } from '/lib/api';\nuseQuery();\napi.load();",
        )])?;
        assert_eq!(report["summary"]["hardFindings"], 1);
        assert_eq!(report["hard"][0]["hasDataFetchPrimitive"], true);
        Ok(())
    }
    #[test]
    fn event_import_is_hard_but_hooks_tests_and_non_presentation_are_excluded(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let report = report_for(&[
            ("src/components/Event.tsx", "import ws from '/lib/ws';"),
            (
                "src/hooks/useOrders.ts",
                "import { api } from '/lib/api'; api.load();",
            ),
            ("src/components/Event.test.tsx", "import ws from '/lib/ws';"),
            ("src/lib/worker.ts", "import ws from '/lib/ws';"),
        ])?;
        assert_eq!(report["summary"]["hardFindings"], 1);
        assert_eq!(report["hard"][0]["kind"], "event-source-import");
        Ok(())
    }
    #[test]
    fn imported_but_uncalled_api_is_not_a_finding() -> Result<(), Box<dyn std::error::Error>> {
        let report = report_for(&[(
            "src/pages/Orders.tsx",
            "import { api } from '/services/orders';\nexport const Orders = () => null;",
        )])?;
        assert_eq!(report["summary"]["totalFindings"], 0);
        Ok(())
    }
}
