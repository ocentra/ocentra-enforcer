//! BOUNDARY-INVARIANT: plan Markdown and proof tables are parsed into typed
//! graph nodes while preserving their source paths and conservative contracts.
//! NEGATIVE-TEST: absent workpacks, unresolved dependencies, and malformed
//! checklist/proof rows remain incomplete rather than being invented.
use super::text::{
    checklist_counts, checklist_nodes, completion_contract, dependency_target, dependency_tokens,
    external_dependency, first_heading, parse_index_row, relative_path, workpack_key, ProofRow,
};
use super::{
    CompletionContract, CyberPlanGraph, EdgeKind, GraphEdge, GraphError, GraphNode, NodeId,
    NodeKind,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

impl CyberPlanGraph {
    pub(crate) fn import_seeds(&mut self) -> Result<(), GraphError> {
        let goal = GraphNode::new(
            self.manifest.goal.id.clone(),
            NodeKind::Goal,
            self.manifest.goal.title.clone(),
            Some(self.manifest.goal.path.clone()),
            CompletionContract::default(),
        );
        let plan = GraphNode::new(
            self.manifest.plan.id.clone(),
            NodeKind::Plan,
            self.manifest.plan.title.clone(),
            Some(self.manifest.plan.path.clone()),
            CompletionContract::default(),
        );
        self.add_node(goal)?;
        self.add_node(plan)?;
        self.add_edge(GraphEdge {
            from: self.manifest.goal.id.clone(),
            to: self.manifest.plan.id.clone(),
            kind: EdgeKind::Contains,
        });
        Ok(())
    }

    pub(crate) fn import_workpacks(&mut self) -> Result<(), GraphError> {
        let root = self.root.join(self.manifest.workpack_root.as_str());
        let proof_rows = self.read_proof_rows()?;
        let mut paths: Vec<PathBuf> = fs::read_dir(root)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md"))
            .collect();
        paths.sort();
        let workpack_ids: BTreeMap<String, NodeId> = paths
            .iter()
            .filter_map(|path| {
                let stem = path.file_stem()?.to_str()?;
                let title = first_heading(&fs::read_to_string(path).ok()?)?;
                let key = workpack_key(&title, stem);
                NodeId::new(format!("WP/{key}")).ok().map(|id| (key, id))
            })
            .collect();
        paths
            .into_iter()
            .try_for_each(|path| self.import_one_workpack(&path, &workpack_ids, &proof_rows))
    }

    pub(crate) fn import_dependency_workpacks(&mut self) -> Result<(), GraphError> {
        let index_path = self
            .manifest
            .dependency_workpack_index
            .as_ref()
            .ok_or_else(|| {
                GraphError::InvalidValue(
                    "dependency workpack import is enabled without an index".to_owned(),
                )
            })?;
        let root_path = self
            .manifest
            .dependency_workpack_root
            .as_ref()
            .ok_or_else(|| {
                GraphError::InvalidValue(
                    "dependency workpack import is enabled without a root".to_owned(),
                )
            })?;
        let index = fs::read_to_string(self.root.join(index_path.as_str()))?;
        let root = self.root.join(root_path.as_str());
        let mut paths: Vec<PathBuf> = fs::read_dir(root)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md"))
            .collect();
        paths.sort();
        let workpack_ids: BTreeMap<String, NodeId> = paths
            .iter()
            .filter_map(|path| {
                let stem = path.file_stem()?.to_str()?;
                let title = first_heading(&fs::read_to_string(path).ok()?)?;
                let key = workpack_key(&title, stem);
                key.starts_with("UL")
                    .then(|| NodeId::new(format!("EXT/{key}")))
                    .and_then(Result::ok)
                    .map(|id| (key, id))
            })
            .collect();
        paths
            .into_iter()
            .try_for_each(|path| self.import_one_dependency_workpack(&path, &workpack_ids, &index))
    }

    pub(crate) fn import_one_dependency_workpack(
        &mut self,
        path: &Path,
        workpack_ids: &BTreeMap<String, NodeId>,
        index: &str,
    ) -> Result<(), GraphError> {
        let contents = fs::read_to_string(path)?;
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                GraphError::InvalidValue("dependency workpack has no UTF-8 stem".to_owned())
            })?;
        let title = first_heading(&contents).unwrap_or_else(|| stem.to_owned());
        let key = workpack_key(&title, stem);
        key.starts_with("UL").then_some(()).ok_or_else(|| {
            GraphError::InvalidValue(format!("dependency workpack `{stem}` is not a UL workpack"))
        })?;
        let id = NodeId::new(format!("EXT/{key}"))?;
        let index_row = parse_index_row(index, &key).ok_or_else(|| {
            GraphError::InvalidValue(format!(
                "dependency workpack `{key}` is missing from its index"
            ))
        })?;
        let relative = relative_path(&self.root, path)?;
        let mut node = GraphNode::new(
            id.clone(),
            NodeKind::Workpack,
            title,
            Some(relative),
            CompletionContract::default(),
        );
        node.metadata
            .insert("routingStatus".to_owned(), index_row.status);
        node.metadata
            .insert("ownerClass".to_owned(), index_row.owner);
        node.metadata
            .insert("batchLimit".to_owned(), index_row.batch_limit);
        node.metadata
            .insert("primaryOwns".to_owned(), index_row.owns);
        node.metadata.insert(
            "dependencyPlan".to_owned(),
            "universal-language-enforcement-plan".to_owned(),
        );
        node.metadata
            .insert("routingOnly".to_owned(), "true".to_owned());
        self.add_node(node)?;
        dependency_tokens(&contents)
            .into_iter()
            .try_for_each(|dependency| {
                let target = dependency_target(&dependency, workpack_ids)?;
                let add_external =
                    target.as_str().starts_with("EXT/") && !self.nodes.contains_key(&target);
                add_external.then_some(()).map_or(Ok(()), |_| {
                    self.add_node(external_dependency(&target, &dependency))
                })?;
                self.add_edge(GraphEdge {
                    from: id.clone(),
                    to: target,
                    kind: EdgeKind::DependsOn,
                });
                Ok::<(), GraphError>(())
            })
    }

    pub(super) fn import_one_workpack(
        &mut self,
        path: &Path,
        workpack_ids: &BTreeMap<String, NodeId>,
        proof_rows: &BTreeMap<String, ProofRow>,
    ) -> Result<(), GraphError> {
        let contents = fs::read_to_string(path)?;
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| GraphError::InvalidValue("workpack has no UTF-8 stem".to_owned()))?;
        let relative = relative_path(&self.root, path)?;
        let title = first_heading(&contents).unwrap_or_else(|| stem.to_owned());
        let key = workpack_key(&title, stem);
        let id = NodeId::new(format!("WP/{key}"))?;
        let index = parse_index_row(
            &fs::read_to_string(self.root.join(self.manifest.workpack_index.as_str()))?,
            &key,
        );
        let proof_row = proof_rows.get(&key);
        let mut completion = completion_contract(proof_row);
        let checklist = checklist_counts(&contents);
        completion.checklist_total = checklist.0;
        completion.checklist_complete = checklist.1;
        let mut node = GraphNode::new(
            id.clone(),
            NodeKind::Workpack,
            title,
            Some(relative),
            completion.clone(),
        );
        index.as_ref().into_iter().for_each(|index| {
            node.metadata
                .insert("routingStatus".to_owned(), index.status.clone());
            node.metadata
                .insert("ownerClass".to_owned(), index.owner.clone());
            node.metadata
                .insert("batchLimit".to_owned(), index.batch_limit.clone());
            node.metadata
                .insert("primaryOwns".to_owned(), index.owns.clone());
        });
        proof_row.into_iter().for_each(|row| {
            node.metadata
                .insert("proofRowState".to_owned(), row.state.clone());
        });
        contents
            .contains("<approved-predicate>")
            .then_some(())
            .into_iter()
            .for_each(|_| {
                node.metadata.insert(
                    "entryApproval".to_owned(),
                    "requires a concrete approved predicate".to_owned(),
                );
            });
        self.add_node(node)?;
        self.add_edge(GraphEdge {
            from: self.manifest.plan.id.clone(),
            to: id.clone(),
            kind: EdgeKind::Contains,
        });
        add_completion_evidence_nodes(self, &completion)?;
        self.add_checklist_edges(&id, &contents)?;
        add_dependency_edges(self, &id, &contents, workpack_ids)?;
        self.add_requirement_edges(&id, &completion);
        Ok(())
    }

    fn add_checklist_edges(&mut self, id: &NodeId, contents: &str) -> Result<(), GraphError> {
        checklist_nodes(id, contents)?
            .into_iter()
            .try_for_each(|requirement| {
                let requirement_id = requirement.id.clone();
                self.add_node(requirement)?;
                self.add_edge(GraphEdge {
                    from: id.clone(),
                    to: requirement_id,
                    kind: EdgeKind::Contains,
                });
                Ok::<(), GraphError>(())
            })
    }

    fn add_requirement_edges(&mut self, id: &NodeId, completion: &CompletionContract) {
        completion
            .required_proofs
            .iter()
            .chain(completion.required_tests.iter())
            .for_each(|evidence| {
                self.add_edge(GraphEdge {
                    from: id.clone(),
                    to: evidence.clone(),
                    kind: EdgeKind::Requires,
                });
            });
    }
}

fn add_completion_evidence_nodes(
    graph: &mut CyberPlanGraph,
    completion: &CompletionContract,
) -> Result<(), GraphError> {
    completion
        .required_tests
        .iter()
        .chain(completion.required_proofs.iter())
        .try_for_each(|evidence_id| {
            (!graph.nodes.contains_key(evidence_id))
                .then_some(())
                .map_or(Ok(()), |_| {
                    let kind = if evidence_id.as_str().starts_with("TEST/") {
                        NodeKind::Test
                    } else {
                        NodeKind::Proof
                    };
                    graph.add_node(GraphNode::new(
                        evidence_id.clone(),
                        kind,
                        "Named completion-contract evidence",
                        None,
                        CompletionContract::default(),
                    ))
                })
        })
}

fn add_dependency_edges(
    graph: &mut CyberPlanGraph,
    id: &NodeId,
    contents: &str,
    workpack_ids: &BTreeMap<String, NodeId>,
) -> Result<(), GraphError> {
    dependency_tokens(contents)
        .into_iter()
        .try_for_each(|dependency| {
            let target = dependency_target(&dependency, workpack_ids)?;
            let add_external =
                target.as_str().starts_with("EXT/") && !graph.nodes.contains_key(&target);
            add_external.then_some(()).map_or(Ok(()), |_| {
                graph.add_node(external_dependency(&target, &dependency))
            })?;
            graph.add_edge(GraphEdge {
                from: id.clone(),
                to: target,
                kind: EdgeKind::DependsOn,
            });
            Ok::<(), GraphError>(())
        })
}
