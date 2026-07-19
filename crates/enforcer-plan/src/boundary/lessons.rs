//! Raw seed-ledger and memory-stream transport decoding.
//!
//! NEGATIVE-TEST: unknown lesson-domain wire values are rejected during decode.

use enforcer_domain::hashes::Sha256;
use enforcer_domain::plan_types::{
    ArtifactRef, CapturedDate, LessonDomain, LessonId, LessonRoute, LessonSequence, LessonText,
    ObservedEvidence, PlanArtifactPath,
};

use crate::boundary::values::{artifact_path, diagnostic_detail};
use crate::error::PlanError;
use crate::lessons::LessonRecord;

pub(crate) fn artifact_error(path: &PlanArtifactPath, reason: impl std::fmt::Display) -> PlanError {
    PlanError::Io {
        path: path.clone(),
        reason: diagnostic_detail(reason.to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LessonRecordWire {
    id: LessonId,
    date: CapturedDate,
    #[serde(with = "lesson_domain_wire")]
    domain: LessonDomain,
    observed: ObservedEvidence,
    lesson: LessonText,
    #[serde(with = "lesson_routes_wire")]
    routes: Vec<LessonRoute>,
    landed_at: Vec<ArtifactRef>,
    supersedes_seq: Option<LessonSequence>,
}

impl From<&LessonRecord> for LessonRecordWire {
    fn from(record: &LessonRecord) -> Self {
        Self {
            id: record.id.clone(),
            date: record.date.clone(),
            domain: record.domain,
            observed: record.observed.clone(),
            lesson: record.lesson.clone(),
            routes: record.routes.clone(),
            landed_at: record.landed_at.clone(),
            supersedes_seq: record.supersedes_seq,
        }
    }
}

impl From<LessonRecordWire> for LessonRecord {
    fn from(record: LessonRecordWire) -> Self {
        Self {
            id: record.id,
            date: record.date,
            domain: record.domain,
            observed: record.observed,
            lesson: record.lesson,
            routes: record.routes,
            landed_at: record.landed_at,
            supersedes_seq: record.supersedes_seq,
        }
    }
}

impl serde::Serialize for LessonRecord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        LessonRecordWire::from(self).serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for LessonRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        LessonRecordWire::deserialize(deserializer).map(Into::into)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct LedgerLine {
    pub(crate) record: LessonRecord,
    pub(crate) digest: Sha256,
}

pub(crate) mod lesson_domain_wire {
    use enforcer_domain::plan_types::LessonDomain;
    use serde::{Deserialize, Deserializer, Serializer};

    pub(crate) fn encode(value: &LessonDomain) -> Option<&'static str> {
        [
            (LessonDomain::Harness, "harness"),
            (LessonDomain::Code, "code"),
        ]
        .into_iter()
        .find_map(|(candidate, wire)| (candidate == *value).then_some(wire))
    }

    pub(crate) fn serialize<S>(value: &LessonDomain, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let wire =
            encode(value).ok_or_else(|| serde::ser::Error::custom("unsupported lesson domain"))?;
        serializer.serialize_str(wire)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<LessonDomain, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        [
            ("harness", LessonDomain::Harness),
            ("code", LessonDomain::Code),
        ]
        .into_iter()
        .find(|(name, _)| *name == raw)
        .map(|(_, value)| value)
        .ok_or_else(|| serde::de::Error::unknown_variant(&raw, &["harness", "code"]))
    }
}

pub(crate) mod lesson_routes_wire {
    use enforcer_domain::plan_types::LessonRoute;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(crate) fn serialize<S>(values: &[LessonRoute], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mapping = [
            (LessonRoute::DoctrineBlock, "doctrineBlock"),
            (LessonRoute::Skill, "skill"),
            (LessonRoute::RuleCandidate, "ruleCandidate"),
            (LessonRoute::ForestNode, "forestNode"),
            (LessonRoute::PlanDoc, "planDoc"),
        ];
        values
            .iter()
            .map(|value| {
                mapping
                    .iter()
                    .find_map(|(candidate, wire)| (candidate == value).then_some(*wire))
                    .ok_or_else(|| serde::ser::Error::custom("unsupported lesson route"))
            })
            .collect::<Result<Vec<_>, S::Error>>()?
            .serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Vec<LessonRoute>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mapping = [
            ("doctrineBlock", LessonRoute::DoctrineBlock),
            ("skill", LessonRoute::Skill),
            ("ruleCandidate", LessonRoute::RuleCandidate),
            ("forestNode", LessonRoute::ForestNode),
            ("planDoc", LessonRoute::PlanDoc),
        ];
        Vec::<String>::deserialize(deserializer)?
            .into_iter()
            .map(|value| {
                mapping
                    .iter()
                    .find_map(|(wire, route)| (wire == &value).then_some(*route))
                    .ok_or_else(|| {
                        serde::de::Error::unknown_variant(
                            &value,
                            &[
                                "doctrineBlock",
                                "skill",
                                "ruleCandidate",
                                "forestNode",
                                "planDoc",
                            ],
                        )
                    })
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SeedRow {
    pub(crate) id: String,
    pub(crate) date: String,
    pub(crate) observed: String,
    pub(crate) lesson: String,
    pub(crate) landed_at: String,
    pub(crate) ships_via: String,
}

pub(crate) fn parse_seed_rows(markdown: &str) -> Vec<SeedRow> {
    markdown.lines().filter_map(parse_seed_row).collect()
}

fn parse_seed_row(line: &str) -> Option<SeedRow> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return None;
    }
    let mut cells = trimmed
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(str::trim);
    let id = cells.next()?;
    if id == "id"
        || id
            .chars()
            .all(|character| character == '-' || character == ':')
        || !id.starts_with('L')
    {
        return None;
    }
    Some(SeedRow {
        id: id.to_owned(),
        date: cells.next()?.to_owned(),
        observed: cells.next()?.to_owned(),
        lesson: cells.next()?.to_owned(),
        landed_at: cells.next()?.to_owned(),
        ships_via: cells.next()?.to_owned(),
    })
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryStreamRecord {
    pub(crate) id: String,
    pub(crate) date: Option<String>,
    pub(crate) domain: Option<String>,
    pub(crate) observed: Option<String>,
    pub(crate) lesson: Option<String>,
    pub(crate) ships_via: Option<String>,
    pub(crate) landed_at: Option<String>,
}

pub(crate) fn decode_memory_stream_record(
    line: &str,
) -> Result<MemoryStreamRecord, serde_json::Error> {
    serde_json::from_str(line)
}

pub(crate) fn render_lesson_template(
    template: &str,
    record: &LessonRecord,
) -> Result<String, PlanError> {
    let bindings = render_bindings(record)?;
    let mut result = template.to_owned();
    for (name, value) in &bindings {
        let placeholder = format!("{{{{{name}}}}}");
        if result.contains(&placeholder) {
            result = result.replace(&placeholder, value);
        }
    }
    if let Some(position) = result.find("{{") {
        let unresolved = result.get(position..).unwrap_or_default();
        if let Some(end) = unresolved.find("}}") {
            let placeholder_length = end.checked_add(2).unwrap_or(end);
            let placeholder = unresolved.get(..placeholder_length).unwrap_or(unresolved);
            return Err(PlanError::Io {
                path: artifact_path("lesson template".into()),
                reason: diagnostic_detail(format!("missing placeholder: {placeholder}")),
            });
        }
    }
    Ok(result)
}

fn render_bindings(
    record: &LessonRecord,
) -> Result<std::collections::HashMap<String, String>, PlanError> {
    let mut bindings = std::collections::HashMap::new();
    bindings.insert("lesson_id".to_owned(), record.id.as_str().to_owned());
    bindings.insert("date".to_owned(), record.date.as_str().to_owned());
    let domain = lesson_domain_wire::encode(&record.domain).ok_or_else(|| PlanError::Io {
        path: artifact_path("lesson template".into()),
        reason: diagnostic_detail("unsupported lesson domain".into()),
    })?;
    bindings.insert("domain".to_owned(), domain.to_owned());
    bindings.insert("observed".to_owned(), record.observed.as_str().to_owned());
    bindings.insert("lesson".to_owned(), record.lesson.as_str().to_owned());
    Ok(bindings)
}

pub(crate) fn replace_or_append_block(existing: &str, new_block: &str, lesson_id: &str) -> String {
    let marker_needle = lesson_id.to_owned();
    let open_marker_line = existing
        .lines()
        .find(|line| line.trim_start().starts_with("<!--") && line.contains(&marker_needle));
    let Some(open_line) = open_marker_line else {
        if existing.is_empty() {
            return new_block.to_owned();
        }
        return format!("{}\n\n{}", existing.trim_end(), new_block);
    };
    let lines: Vec<&str> = existing.lines().collect();
    let Some(open_index) = lines.iter().position(|line| *line == open_line) else {
        return format!("{}\n\n{}", existing.trim_end(), new_block);
    };
    let Some(lines_from_open) = lines.get(open_index..) else {
        return format!("{}\n\n{}", existing.trim_end(), new_block);
    };
    let close_index = lines_from_open
        .iter()
        .position(|line| line.trim_start().starts_with("<!-- /") && line.contains(&marker_needle))
        .and_then(|offset| open_index.checked_add(offset));
    let Some(close_index) = close_index else {
        return format!("{}\n\n{}", existing.trim_end(), new_block);
    };
    let Some(after_close_start) = close_index.checked_add(1) else {
        return format!("{}\n\n{}", existing.trim_end(), new_block);
    };
    let Some(before_open) = lines.get(..open_index) else {
        return format!("{}\n\n{}", existing.trim_end(), new_block);
    };
    let Some(after_close) = lines.get(after_close_start..) else {
        return format!("{}\n\n{}", existing.trim_end(), new_block);
    };
    let mut output_lines: Vec<&str> = Vec::new();
    output_lines.extend_from_slice(before_open);
    output_lines.extend(new_block.lines());
    output_lines.extend_from_slice(after_close);
    let mut output = output_lines.join("\n");
    output.push('\n');
    output
}

#[cfg(test)]
mod tests {
    use super::lesson_domain_wire;

    #[test]
    fn lesson_domain_wire_rejects_an_unknown_value() {
        let mut decoder = serde_json::Deserializer::from_str("\"unknown\"");
        let result = lesson_domain_wire::deserialize(&mut decoder);
        assert!(matches!(
            result,
            Err(error) if error.classify() == serde_json::error::Category::Data
        ));
    }
}
