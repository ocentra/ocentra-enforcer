//! BOUNDARY-INVARIANT: proof and evidence files are imported as provenance
//! metadata only; they never grant native implementation or executable proof.
//! NEGATIVE-TEST: missing, malformed, protected, or contradictory evidence is
//! retained as a graph issue and cannot be opened as a vendor source file.
use super::json::{
    array_field, packet_issue, partition_issue, string_array, string_field, valid_cp11_skill,
};
use super::text::{parse_proof_row, relative_path, ProofRow};
use super::{
    CompletionContract, CyberPlanGraph, EdgeKind, GraphEdge, GraphError, GraphIssue, GraphNode,
    IssueLevel, NodeId, NodeKind, CYBERSKILLS_REGISTRY_PATH, PROTECTED_SKILL,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

impl CyberPlanGraph {
    pub(super) fn read_proof_rows(&self) -> Result<BTreeMap<String, ProofRow>, GraphError> {
        let path = self
            .root
            .join(self.manifest.test_proof_expectations.as_str());
        let text = fs::read_to_string(path)?;
        Ok(text
            .lines()
            .filter_map(parse_proof_row)
            .map(|row| (row.workpack.clone(), row))
            .collect())
    }

    pub(crate) fn import_cp01_proofs(&mut self) -> Result<(), GraphError> {
        let Some(proof_root) = self.manifest.proof_roots.first() else {
            return Ok(());
        };
        let root = self.root.join(proof_root.as_str()).join("cp01");
        let paths = sorted_evidence_paths(&root, "reconciliation.json")?;
        paths
            .iter()
            .try_for_each(|path| self.import_one_cp01_proof(path))?;
        self.validate_cp01_partition(&paths)
    }

    fn import_one_cp01_proof(&mut self, path: &Path) -> Result<(), GraphError> {
        let evidence: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
        let batch = string_field(&evidence, &["batch"]).unwrap_or_else(|| "unknown".to_owned());
        let id = NodeId::new(format!("PROOF/CP01/{batch}"))?;
        let relative = relative_path(&self.root, path)?;
        let mut node = GraphNode::new(
            id.clone(),
            NodeKind::Proof,
            format!("CP01 reconciliation evidence {batch}"),
            Some(relative),
            CompletionContract::default(),
        );
        array_field(&evidence, &["rules"])
            .map(Vec::len)
            .into_iter()
            .for_each(|count| {
                node.metadata
                    .insert("ruleCount".to_owned(), count.to_string());
            });
        self.add_node(node)?;
        self.add_edge(GraphEdge {
            from: NodeId::new("WP/CP01")?,
            to: id,
            kind: EdgeKind::Produces,
        });
        Ok(())
    }

    fn validate_cp01_partition(&mut self, paths: &[PathBuf]) -> Result<(), GraphError> {
        let registry: Value = serde_json::from_str(&fs::read_to_string(
            self.root.join(CYBERSKILLS_REGISTRY_PATH),
        )?)?;
        let expected: BTreeSet<String> = array_field(&registry, &[])
            .into_iter()
            .flat_map(|rules| rules.iter())
            .filter_map(|rule| string_field(rule, &["ruleId"]))
            .collect();
        let mut seen = BTreeSet::new();
        paths.iter().try_for_each(|path| {
            let evidence: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
            array_field(&evidence, &["rules"])
                .into_iter()
                .flat_map(|rules| rules.iter())
                .filter_map(|rule| string_field(rule, &["ruleId"]))
                .for_each(|rule_id| {
                    record_cp01_duplicate(&mut self.issues, seen.insert(rule_id.clone()), &rule_id);
                });
            Ok::<(), GraphError>(())
        })?;
        let missing: Vec<&String> = expected.difference(&seen).collect();
        let extra: Vec<&String> = seen.difference(&expected).collect();
        (!missing.is_empty() || !extra.is_empty() || expected.len() != seen.len())
            .then_some(())
            .into_iter()
            .for_each(|_| {
                self.issues.push(partition_issue(
                    "CP01-REGISTRY-PARTITION",
                    format!(
                        "CP01 registry partition mismatch: expected {}, covered {}, missing {:?}, extra {:?}",
                        expected.len(),
                        seen.len(),
                        missing,
                        extra
                    ),
                ));
            });
        Ok(())
    }

    pub(crate) fn import_cp11_proofs(&mut self) -> Result<(), GraphError> {
        let Some(proof_root) = self.manifest.proof_roots.first() else {
            return Ok(());
        };
        let root = self.root.join(proof_root.as_str()).join("cp11");
        let paths = sorted_evidence_paths(&root, "retention.json")?;
        let mut family_packet_numbers = BTreeMap::<String, usize>::new();
        paths.iter().try_for_each(|path| {
            let evidence: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
            let family = string_field(&evidence, &["family"]).ok_or_else(|| {
                GraphError::InvalidValue(format!(
                    "CP11 retention artifact `{}` has no family",
                    path.display()
                ))
            })?;
            let packet_number = family_packet_numbers
                .entry(family)
                .and_modify(|number| *number += 1)
                .or_insert(1);
            self.import_one_cp11_proof(path, *packet_number)?;
            self.validate_cp11_packet(path)
        })
    }

    fn import_one_cp11_proof(
        &mut self,
        path: &Path,
        packet_number: usize,
    ) -> Result<(), GraphError> {
        let evidence: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
        let batch = string_field(&evidence, &["batch"]).unwrap_or_else(|| "unknown".to_owned());
        let family = string_field(&evidence, &["family"]).ok_or_else(|| {
            GraphError::InvalidValue(format!(
                "CP11 retention artifact `{}` has no family",
                path.display()
            ))
        })?;
        let family_id = family.replace('/', "-");
        let id = NodeId::new(format!("PROOF/CP11/{batch}"))?;
        let relative = relative_path(&self.root, path)?;
        let mut node = GraphNode::new(
            id.clone(),
            NodeKind::Proof,
            format!("CP11 retention evidence {batch}"),
            Some(relative.clone()),
            CompletionContract::default(),
        );
        array_field(&evidence, &["skills"])
            .map(Vec::len)
            .into_iter()
            .for_each(|count| {
                node.metadata
                    .insert("skillCount".to_owned(), count.to_string());
            });
        self.add_node(node)?;
        self.add_edge(GraphEdge {
            from: NodeId::new("WP/CP11")?,
            to: id,
            kind: EdgeKind::Produces,
        });
        let gate_id = NodeId::new(format!("TEST/WP/CP11/{family_id}/B{packet_number:02}/gate"))?;
        let gate_node = GraphNode::new(
            gate_id.clone(),
            NodeKind::Test,
            format!("CP11 retention gate {family_id} B{packet_number:02}"),
            Some(relative),
            CompletionContract::default(),
        );
        self.add_node(gate_node)?;
        self.add_edge(GraphEdge {
            from: NodeId::new("WP/CP11")?,
            to: gate_id,
            kind: EdgeKind::Produces,
        });
        Ok(())
    }

    fn validate_cp11_packet(&mut self, path: &Path) -> Result<(), GraphError> {
        let evidence: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
        let Some(skills) = array_field(&evidence, &["skills"]) else {
            self.issues.push(packet_issue("CP11-SKILLS-MISSING", path));
            return Ok(());
        };
        let mut ids = BTreeSet::new();
        skills
            .iter()
            .filter(|skill| !valid_cp11_skill(&self.root, skill, &mut ids))
            .for_each(|_| self.issues.push(packet_issue("CP11-SKILL-BOUNDARY", path)));
        (ids.len() != skills.len() || skills.is_empty() || skills.len() > 10)
            .then_some(())
            .into_iter()
            .for_each(|_| self.issues.push(packet_issue("CP11-PACKET-SIZE", path)));
        Ok(())
    }

    pub(crate) fn import_cp08_proofs(&mut self) -> Result<(), GraphError> {
        let Some(proof_root) = self.manifest.proof_roots.first() else {
            return Ok(());
        };
        let root = self.root.join(proof_root.as_str()).join("cp08");
        if !root.exists() {
            return Ok(());
        }
        let mut paths: Vec<PathBuf> = fs::read_dir(root)?
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("decomposition.json"))
            .filter(|path| path.is_file())
            .collect();
        paths.sort();
        for path in &paths {
            self.import_one_cp08_proof(path)?;
        }
        self.validate_cp08_retention(&paths)?;
        Ok(())
    }

    fn validate_cp08_retention(&mut self, paths: &[PathBuf]) -> Result<(), GraphError> {
        let mut catalog_ids = BTreeSet::new();
        paths
            .iter()
            .try_for_each(|path| self.validate_cp08_artifact(path, &mut catalog_ids))?;
        (catalog_ids.len() != 816)
            .then_some(())
            .into_iter()
            .for_each(|_| {
                self.issues.push(GraphIssue {
                    level: IssueLevel::Error,
                    code: "CP11-RETENTION-COUNT".to_owned(),
                    node: None,
                    message: format!(
                        "retention evidence covers {} catalog IDs; expected 816",
                        catalog_ids.len()
                    ),
                });
            });
        Ok(())
    }

    fn validate_cp08_artifact(
        &mut self,
        path: &Path,
        catalog_ids: &mut BTreeSet<String>,
    ) -> Result<(), GraphError> {
        let evidence: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
        array_field(&evidence, &["skills"])
            .into_iter()
            .flat_map(|skills| skills.iter())
            .try_for_each(|skill| self.validate_cp08_skill(path, skill, catalog_ids))
    }

    fn validate_cp08_skill(
        &mut self,
        path: &Path,
        skill: &Value,
        catalog_ids: &mut BTreeSet<String>,
    ) -> Result<(), GraphError> {
        let Some(catalog_id) = string_field(skill, &["catalogId"]) else {
            self.issues.push(GraphIssue {
                level: IssueLevel::Error,
                code: "CP11-CATALOG-ID-MISSING".to_owned(),
                node: None,
                message: format!(
                    "CP08 artifact `{}` contains a skill without catalogId",
                    path.display()
                ),
            });
            return Ok(());
        };
        let node = NodeId::new(format!("SKILL/{catalog_id}"))?;
        (!catalog_ids.insert(catalog_id.clone()))
            .then_some(())
            .into_iter()
            .for_each(|_| {
                self.issues.push(GraphIssue {
                    level: IssueLevel::Error,
                    code: "CP11-DUPLICATE-SKILL".to_owned(),
                    node: Some(node.clone()),
                    message: format!("retention evidence repeats catalog ID `{catalog_id}`"),
                });
            });
        let source_valid = string_field(skill, &["source", "path"])
            .is_some_and(|value| !value.trim().is_empty())
            && string_field(skill, &["source", "sha256"]).is_some_and(|value| value.len() == 64)
            && string_field(skill, &["source", "license"])
                .is_some_and(|value| value == "Apache-2.0")
            && !string_array(skill, &["source", "anchors"]).is_empty();
        (!source_valid).then_some(()).into_iter().for_each(|_| {
            self.issues.push(GraphIssue {
                level: IssueLevel::Error,
                code: "CP11-SOURCE-EVIDENCE-MISSING".to_owned(),
                node: Some(node.clone()),
                message: format!(
                    "retention evidence for `{catalog_id}` lacks source path/hash/license/anchors"
                ),
            });
        });
        validate_retained_components(
            &mut self.issues,
            &node,
            &catalog_id,
            array_field(skill, &["components"]),
        );
        Ok(())
    }

    fn import_one_cp08_proof(&mut self, path: &Path) -> Result<(), GraphError> {
        let evidence: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
        self.record_cp08_component_kinds(&evidence);
        let batch = string_field(&evidence, &["batch"]).unwrap_or_else(|| "unknown".to_owned());
        let id = NodeId::new(format!("PROOF/CP08/{batch}"))?;
        let relative = relative_path(&self.root, path)?;
        let mut node = GraphNode::new(
            id.clone(),
            NodeKind::Proof,
            format!("CP08 decomposition evidence {batch}"),
            Some(relative),
            CompletionContract::default(),
        );
        array_field(&evidence, &["selection", "catalogIds"])
            .map(Vec::len)
            .into_iter()
            .for_each(|count| {
                node.metadata
                    .insert("catalogCount".to_owned(), count.to_string());
            });
        self.add_node(node)?;
        let workpack = NodeId::new("WP/CP08")?;
        self.add_edge(GraphEdge {
            from: workpack,
            to: id.clone(),
            kind: EdgeKind::Produces,
        });
        string_array(&evidence, &["selection", "catalogIds"])
            .into_iter()
            .filter_map(|catalog_id| NodeId::new(format!("SKILL/{catalog_id}")).ok())
            .for_each(|skill| {
                self.add_edge(GraphEdge {
                    from: id.clone(),
                    to: skill,
                    kind: EdgeKind::EvidenceFor,
                });
            });
        Ok(())
    }
    fn record_cp08_component_kinds(&mut self, evidence: &Value) {
        array_field(evidence, &["skills"])
            .into_iter()
            .flat_map(|skills| skills.iter())
            .filter_map(|skill| string_field(skill, &["catalogId"]).map(|id| (id, skill)))
            .filter(|(catalog_id, _)| catalog_id != PROTECTED_SKILL)
            .for_each(|(catalog_id, skill)| {
                array_field(skill, &["components"])
                    .into_iter()
                    .flat_map(|components| components.iter())
                    .filter_map(|component| string_field(component, &["kind"]))
                    .for_each(|kind| {
                        self.cp08_component_kinds
                            .entry(catalog_id.clone())
                            .or_default()
                            .insert(kind);
                    });
            });
    }
}

fn validate_retained_components(
    issues: &mut Vec<GraphIssue>,
    node: &NodeId,
    catalog_id: &str,
    components: Option<&Vec<Value>>,
) {
    ["advisory", "manual"].into_iter().for_each(|kind| {
        let retained = components
            .into_iter()
            .flat_map(|values| values.iter())
            .filter(|component| retained_component_is_valid(component, kind))
            .count();
        (retained != 1).then_some(()).into_iter().for_each(|_| {
            issues.push(GraphIssue {
                level: IssueLevel::Error,
                code: "CP11-RETENTION-KIND".to_owned(),
                node: Some(node.clone()),
                message: format!(
                    "`{catalog_id}` must have exactly one retained {kind} component with purpose and notProved"
                ),
            });
        });
    });
}

fn record_cp01_duplicate(issues: &mut Vec<GraphIssue>, inserted: bool, rule_id: &str) {
    (!inserted)
        .then_some(partition_issue(
            "CP01-DUPLICATE-RULE",
            format!("CP01 evidence repeats registry rule `{rule_id}`"),
        ))
        .into_iter()
        .for_each(|issue| issues.push(issue));
}

fn retained_component_is_valid(component: &Value, kind: &str) -> bool {
    string_field(component, &["kind"]).as_deref() == Some(kind)
        && string_field(component, &["status"]).as_deref() == Some("retained")
        && string_field(component, &["predicateOrPurpose"])
            .is_some_and(|value| !value.trim().is_empty())
        && valid_not_proved(component)
}

fn valid_not_proved(component: &Value) -> bool {
    component
        .get("notProved")
        .and_then(Value::as_array)
        .is_some_and(|values| {
            !values.is_empty()
                && values
                    .iter()
                    .all(|value| value.as_str().is_some_and(|text| !text.trim().is_empty()))
        })
}

fn sorted_evidence_paths(root: &Path, filename: &str) -> Result<Vec<PathBuf>, GraphError> {
    let mut paths = root
        .is_dir()
        .then(|| fs::read_dir(root))
        .transpose()?
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path().join(filename))
                .filter(|path| path.is_file())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    paths.sort();
    Ok(paths)
}
