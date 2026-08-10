//! BOUNDARY-INVARIANT: Markdown and table text is parsed into small typed
//! records; parser uncertainty remains conservative graph metadata.
//! NEGATIVE-TEST: malformed headings, rows, paths, and dependency tokens do
//! not become trusted completion evidence.
use super::{CompletionContract, GraphError, GraphNode, GraphPath, NodeId, NodeKind};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) struct ProofRow {
    pub(super) workpack: String,
    pub(super) proof: String,
    pub(super) gates: String,
    pub(super) state: String,
}

#[derive(Debug)]
pub(super) struct IndexRow {
    pub(super) status: String,
    pub(super) owner: String,
    pub(super) batch_limit: String,
    pub(super) owns: String,
}
pub(crate) fn relative_path(root: &Path, path: &Path) -> Result<GraphPath, GraphError> {
    let relative = path.strip_prefix(root).map_err(|_| {
        GraphError::InvalidValue("evidence path escaped repository root".to_owned())
    })?;
    GraphPath::new(relative.to_string_lossy().into_owned())
}

pub(crate) fn first_heading(text: &str) -> Option<String> {
    text.lines()
        .find(|line| line.starts_with("# "))
        .map(|line| line.trim_start_matches("# ").trim().to_owned())
}

pub(crate) fn workpack_key(title: &str, stem: &str) -> String {
    title
        .split_whitespace()
        .next()
        .filter(|value| {
            let value = value.to_ascii_uppercase();
            value.starts_with("CP") || value.starts_with("UL")
        })
        .unwrap_or(stem)
        .to_ascii_uppercase()
}

pub(crate) fn backtick_values(text: &str) -> Vec<String> {
    text.split('`')
        .enumerate()
        .filter_map(|(index, value)| (index % 2 == 1).then_some(value.trim().to_owned()))
        .filter(|value| !value.is_empty())
        .collect()
}

pub(crate) fn dependency_tokens(text: &str) -> Vec<String> {
    let Some(line) = text
        .lines()
        .find(|line| line.trim_start().starts_with("- deps:"))
    else {
        return Vec::new();
    };
    let values = backtick_values(line);
    if values
        .iter()
        .any(|value| value.eq_ignore_ascii_case("none"))
    {
        return Vec::new();
    }
    values
        .into_iter()
        .flat_map(|value| {
            value
                .split(',')
                .map(str::trim)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|value| !value.is_empty())
        .collect()
}

pub(crate) fn dependency_target(
    raw: &str,
    workpack_ids: &BTreeMap<String, NodeId>,
) -> Result<NodeId, GraphError> {
    let key = raw.trim().to_ascii_uppercase();
    if let Some(target) = workpack_ids.get(&key) {
        return Ok(target.clone());
    }
    if key.starts_with("UL") {
        return NodeId::new(format!("EXT/{key}"));
    }
    NodeId::new(format!("MISSING/{key}"))
}

pub(crate) fn external_dependency(id: &NodeId, raw: &str) -> GraphNode {
    let mut node = GraphNode::new(
        id.clone(),
        NodeKind::Dependency,
        format!("External dependency {raw}"),
        None,
        CompletionContract::default(),
    );
    node.metadata
        .insert("authority".to_owned(), "external-plan".to_owned());
    node
}

pub(crate) fn checklist_counts(text: &str) -> (usize, usize) {
    text.lines()
        .filter(|line| line.trim_start().starts_with("- [") || line.trim_start().starts_with("* ["))
        .fold((0, 0), |(total, complete), line| {
            (
                total + 1,
                complete + usize::from(line.contains("[x]") || line.contains("[X]")),
            )
        })
}

pub(crate) fn checklist_nodes(id: &NodeId, text: &str) -> Result<Vec<GraphNode>, GraphError> {
    text.lines()
        .filter(|line| line.trim_start().starts_with("- [") || line.trim_start().starts_with("* ["))
        .enumerate()
        .map(|(index, line)| {
            let content = line
                .split_once(']')
                .map(|(_, value)| value.trim())
                .unwrap_or(line.trim());
            let slug = stable_slug(content);
            let requirement_id = NodeId::new(format!("{id}/REQ/{slug}-{index}"))?;
            let mut node = GraphNode::new(
                requirement_id,
                NodeKind::Requirement,
                content,
                None,
                CompletionContract::default(),
            );
            node.metadata.insert(
                "checked".to_owned(),
                (line.contains("[x]") || line.contains("[X]")).to_string(),
            );
            Ok(node)
        })
        .collect()
}

pub(crate) fn stable_slug(text: &str) -> String {
    let mut slug = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "item".to_owned()
    } else {
        trimmed.chars().take(72).collect()
    }
}

pub(crate) fn parse_index_row(index: &str, stem: &str) -> Option<IndexRow> {
    index.lines().find_map(|line| {
        if !line.trim_start().starts_with('|') {
            return None;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() < 8 || !cells.get(2)?.eq_ignore_ascii_case(stem) {
            return None;
        }
        Some(IndexRow {
            status: cells.get(1)?.to_string(),
            owner: cells.get(4)?.to_string(),
            batch_limit: cells.get(6)?.to_string(),
            owns: cells.get(7)?.to_string(),
        })
    })
}

pub(crate) fn parse_proof_row(line: &str) -> Option<ProofRow> {
    if !line.trim_start().starts_with('|') || line.contains("---") {
        return None;
    }
    let cells: Vec<&str> = line.split('|').map(str::trim).collect();
    if cells.len() < 5 || !cells.get(1)?.to_ascii_uppercase().starts_with("CP") {
        return None;
    }
    Some(ProofRow {
        workpack: cells.get(1)?.to_ascii_uppercase(),
        proof: cells.get(2)?.to_string(),
        gates: cells.get(3)?.to_string(),
        state: cells.get(4)?.to_string(),
    })
}

pub(crate) fn completion_contract(row: Option<&ProofRow>) -> CompletionContract {
    let Some(row) = row else {
        return CompletionContract::default();
    };
    let required_proofs: Vec<NodeId> = backtick_values(&row.proof)
        .into_iter()
        .filter(|path| path.contains('/') || path.ends_with(".json"))
        .filter_map(|path| NodeId::new(format!("PROOF/PATH/{}", path.replace('/', "_"))).ok())
        .collect();
    let required_tests: Vec<NodeId> = row
        .gates
        .split(';')
        .map(str::trim)
        .filter(|gate| !gate.is_empty())
        .filter_map(|gate| NodeId::new(format!("TEST/{}/{}", row.workpack, stable_slug(gate))).ok())
        .collect();
    CompletionContract {
        required_paths: Vec::new(),
        required_tests,
        required_proofs,
        required_adrs: Vec::new(),
        checklist_total: 0,
        checklist_complete: 0,
    }
}
