//! Process-local compatibility history for native MCP validation results.
//!
//! Mirrors frozen MJS `validationHistory`: case-folded resolved roots,
//! no persistence, and the newest twenty scan/check summaries per root.

use std::collections::{BTreeMap, VecDeque};

use enforcer_core::platform::{epoch_millis, iso8601_utc};

const HISTORY_LIMIT: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationKind {
    Scan,
    Check,
}

impl ValidationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scan => "scan",
            Self::Check => "check",
        }
    }
}

#[derive(Debug, Default)]
pub struct ValidationHistory {
    by_root: BTreeMap<String, VecDeque<serde_json::Value>>,
}

impl ValidationHistory {
    pub fn record(&mut self, root: &str, kind: ValidationKind, report: &serde_json::Value) {
        let entries = self.by_root.entry(root.to_lowercase()).or_default();
        entries.push_front(summary(root, kind, report));
        entries.truncate(HISTORY_LIMIT);
    }

    pub fn latest(&self, root: &str, tool: Option<&str>) -> Option<serde_json::Value> {
        let kind = match tool {
            Some("check") => Some("check"),
            Some("scan") => Some("scan"),
            _ => None,
        };
        self.by_root
            .get(&root.to_lowercase())?
            .iter()
            .find(|entry| kind.is_none_or(|kind| entry["kind"] == kind))
            .cloned()
    }
}

fn summary(root: &str, kind: ValidationKind, report: &serde_json::Value) -> serde_json::Value {
    let findings = ["violations", "warnings"]
        .into_iter()
        .flat_map(|field| {
            report
                .get(field)
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
        })
        .collect::<Vec<_>>();
    let mut severity = serde_json::Map::new();
    for finding in &findings {
        if let Some(value) = finding.get("severity").and_then(serde_json::Value::as_str) {
            let entry = severity
                .entry(value.to_owned())
                .or_insert_with(|| serde_json::json!(0));
            *entry = serde_json::json!(entry.as_u64().unwrap_or(0) + 1);
        }
    }
    let values = |field: &str| {
        let mut values = findings
            .iter()
            .filter_map(|finding| finding.get(field).and_then(serde_json::Value::as_str))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        values.sort();
        values.dedup();
        values
    };
    let mut object = serde_json::Map::new();
    object.insert("kind".to_owned(), serde_json::json!(kind.as_str()));
    object.insert(
        "command".to_owned(),
        report
            .get("command")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "check".to_owned(),
        report
            .get("check")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "ok".to_owned(),
        report.get("ok").cloned().unwrap_or(serde_json::Value::Null),
    );
    object.insert("root".to_owned(), serde_json::json!(root));
    object.insert(
        "profileName".to_owned(),
        report
            .get("profileName")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "at".to_owned(),
        serde_json::json!(epoch_millis()
            .map(iso8601_utc)
            .unwrap_or_else(|_| "1970-01-01T00:00:00.000Z".to_owned())),
    );
    object.insert(
        "bySeverity".to_owned(),
        report
            .get("bySeverity")
            .cloned()
            .unwrap_or(serde_json::Value::Object(severity)),
    );
    object.insert("counts".to_owned(), serde_json::json!({ "findings": findings.len(), "violations": report.get("violations").and_then(serde_json::Value::as_array).map_or(0, Vec::len), "warnings": report.get("warnings").and_then(serde_json::Value::as_array).map_or(0, Vec::len) }));
    object.insert("ruleIds".to_owned(), serde_json::json!(values("ruleId")));
    object.insert("docs".to_owned(), serde_json::json!(values("doc")));
    object.insert("scope".to_owned(), compact_scope(report.get("scope")));
    for field in ["command", "check", "profileName", "scope"] {
        if object.get(field).is_some_and(serde_json::Value::is_null) {
            object.remove(field);
        }
    }
    serde_json::Value::Object(object)
}

fn compact_scope(scope: Option<&serde_json::Value>) -> serde_json::Value {
    let Some(scope) = scope else {
        return serde_json::Value::Null;
    };
    let mut compact = serde_json::Map::new();
    for field in ["mode", "crateName", "base", "head"] {
        if let Some(value) = scope.get(field) {
            compact.insert(field.to_owned(), value.clone());
        }
    }
    if let Some(files) = scope.get("files").and_then(serde_json::Value::as_array) {
        compact.insert("fileCount".to_owned(), serde_json::json!(files.len()));
        compact.insert(
            "sampleFiles".to_owned(),
            serde_json::Value::Array(files.iter().take(20).cloned().collect()),
        );
    }
    serde_json::Value::Object(compact)
}

#[cfg(test)]
mod tests {
    use super::{ValidationHistory, ValidationKind, HISTORY_LIMIT};
    fn report(id: &str) -> serde_json::Value {
        serde_json::json!({"ok":false,"violations":[{"ruleId":id,"doc":"rules/test","severity":"error"}],"warnings":[]})
    }
    #[test]
    fn retains_newest_twenty_case_folded_per_root_and_filters_kind() {
        let mut history = ValidationHistory::default();
        history.record("C:/Repo", ValidationKind::Scan, &report("SCAN-1"));
        history.record("c:/repo", ValidationKind::Check, &report("CHECK-1"));
        for index in 0..HISTORY_LIMIT {
            history.record(
                "C:/OTHER",
                ValidationKind::Scan,
                &report(&format!("R-{index}")),
            );
        }
        assert_eq!(
            history.latest("C:/REPO", Some("check")).unwrap()["kind"],
            "check"
        );
        assert_eq!(
            history.latest("c:/repo", Some("scan")).unwrap()["kind"],
            "scan"
        );
        assert_eq!(history.by_root["c:/other"].len(), HISTORY_LIMIT);
    }
}
