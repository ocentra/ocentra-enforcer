//! BOUNDARY-INVARIANT: intent-matrix rows are partitioned into bounded packets
//! using CP08 component evidence without promoting advisory or external rows.
//! NEGATIVE-TEST: duplicate, unknown, and unassigned catalog identities reject
//! the graph rather than creating an implicit workpack.
use super::json::{
    repository_graph_skill_ids, required_string_array, string_array, string_field, usize_field,
    validate_intent_matrix_header,
};
use super::{
    CompletionContract, CyberPlanGraph, EdgeKind, GraphEdge, GraphError, GraphNode, GraphPath,
    NodeId, NodeKind, PROTECTED_SKILL,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;

impl CyberPlanGraph {
    pub(super) fn import_intent_matrix(&mut self) -> Result<(), GraphError> {
        let path = self
            .manifest
            .intent_matrix_path
            .as_ref()
            .ok_or_else(|| {
                GraphError::InvalidValue(
                    "intent matrix import is enabled without a path".to_owned(),
                )
            })?
            .clone();
        let matrix: Value =
            serde_json::from_str(&fs::read_to_string(self.root.join(path.as_str()))?)?;
        validate_intent_matrix_header(&matrix)?;
        let families = matrix
            .get("families")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                GraphError::InvalidValue("intent matrix families are missing".to_owned())
            })?;
        let repository_graph_skill_ids = repository_graph_skill_ids(&matrix)?;
        let mut assigned = BTreeSet::new();
        let mut owned_components = BTreeSet::new();
        families.iter().try_for_each(|family| {
            self.import_intent_family(
                family,
                &path,
                &repository_graph_skill_ids,
                &mut assigned,
                &mut owned_components,
            )
        })?;
        repository_graph_skill_ids
            .iter()
            .find(|skill_id| !assigned.contains(*skill_id))
            .map_or(Ok(()), |skill_id| {
                Err(GraphError::InvalidValue(format!(
                    "repository graph qualification references unknown or unassigned skill `{skill_id}`"
                )))
            })?;
        self.validate_intent_partition(&assigned)?;
        Ok(())
    }

    fn import_intent_family(
        &mut self,
        family: &Value,
        matrix_path: &GraphPath,
        repository_graph_skill_ids: &BTreeSet<String>,
        assigned: &mut BTreeSet<String>,
        owned_components: &mut BTreeSet<String>,
    ) -> Result<(), GraphError> {
        let family_id = string_field(family, &["familyId"])
            .ok_or_else(|| GraphError::InvalidValue("intent family has no familyId".to_owned()))?;
        let intent = string_field(family, &["intent"]).ok_or_else(|| {
            GraphError::InvalidValue(format!("intent family `{family_id}` has no intent"))
        })?;
        let skill_ids = required_string_array(family, &["skillIds"], &family_id)?;
        let family_node = NodeId::new(family_id.clone())?;
        let mut node = GraphNode::new(
            family_node.clone(),
            NodeKind::IntentFamily,
            intent,
            Some(matrix_path.clone()),
            CompletionContract {
                required_paths: vec![matrix_path.clone()],
                ..CompletionContract::default()
            },
        );
        node.metadata
            .insert("familyId".to_owned(), family_id.clone());
        node.metadata
            .insert("skillCount".to_owned(), skill_ids.len().to_string());
        node.metadata.insert(
            "nativeRoute".to_owned(),
            string_field(family, &["nativeRoute"]).unwrap_or_else(|| "CP09".to_owned()),
        );
        self.add_node(node)?;
        self.add_edge(GraphEdge {
            from: self.manifest.plan.id.clone(),
            to: family_node.clone(),
            kind: EdgeKind::Contains,
        });
        skill_ids.iter().try_for_each(|skill_id| {
            self.classify_intent_skill(&family_node, skill_id, assigned)
        })?;
        self.derive_family_packets(
            &family_node,
            &family_id,
            matrix_path,
            &skill_ids,
            repository_graph_skill_ids,
            owned_components,
            family,
        )
    }

    fn derive_family_packets(
        &mut self,
        family_node: &NodeId,
        family_id: &str,
        matrix_path: &GraphPath,
        skill_ids: &[String],
        repository_graph_skill_ids: &BTreeSet<String>,
        owned_components: &mut BTreeSet<String>,
        family: &Value,
    ) -> Result<(), GraphError> {
        let native_route =
            string_field(family, &["nativeRoute"]).unwrap_or_else(|| "CP09".to_owned());
        let native_limit = usize_field(family, &["nativeBatchLimit"]).unwrap_or(5);
        let native_dependencies = string_array(family, &["dependencies"]);
        let native_skill_ids = self.native_skill_ids(&skill_ids);
        if native_route == "CP12" {
            let (graph_skills, static_skills): (Vec<_>, Vec<_>) = native_skill_ids
                .iter()
                .cloned()
                .partition(|skill_id| repository_graph_skill_ids.contains(skill_id));
            self.derive_intent_packets(
                &family_node,
                &family_id,
                matrix_path,
                &static_skills,
                "CP09",
                5,
                &["WP/CP05".to_owned(), "WP/CP08".to_owned()],
                "native-predicate",
                owned_components,
            )?;
            self.derive_intent_packets(
                &family_node,
                &family_id,
                matrix_path,
                &graph_skills,
                "CP12",
                native_limit,
                &native_dependencies,
                "native-predicate",
                owned_components,
            )?;
        } else {
            self.derive_intent_packets(
                &family_node,
                &family_id,
                matrix_path,
                &native_skill_ids,
                &native_route,
                native_limit,
                &native_dependencies,
                "native-predicate",
                owned_components,
            )?;
        }
        self.derive_intent_packets(
            &family_node,
            &family_id,
            matrix_path,
            &skill_ids,
            "CP11",
            10,
            &["WP/CP08".to_owned()],
            "advisory-manual",
            owned_components,
        )?;
        Ok(())
    }

    fn native_skill_ids(&self, skill_ids: &[String]) -> Vec<String> {
        skill_ids
            .iter()
            .filter(|skill_id| {
                self.cp08_component_kinds
                    .get(*skill_id)
                    .is_some_and(|kinds| kinds.contains("native-predicate"))
            })
            .cloned()
            .collect()
    }

    fn classify_intent_skill(
        &mut self,
        family_node: &NodeId,
        skill_id: &str,
        assigned: &mut BTreeSet<String>,
    ) -> Result<(), GraphError> {
        let skill_node = NodeId::new(format!("SKILL/{skill_id}"))?;
        let skill = self.nodes.get(&skill_node).ok_or_else(|| {
            GraphError::InvalidValue(format!(
                "intent matrix references unknown skill `{skill_id}`"
            ))
        })?;
        if skill.metadata.get("sourceAvailability").map(String::as_str) == Some("sourceUnavailable")
            || skill_id == PROTECTED_SKILL
        {
            return Err(GraphError::InvalidValue(
                "protected sourceUnavailable skill appears in intent matrix".to_owned(),
            ));
        }
        (!assigned.insert(skill_id.to_owned()))
            .then_some(())
            .map_or(Ok(()), |_| {
                Err(GraphError::InvalidValue(format!(
                    "intent matrix assigns `{skill_id}` to more than one family"
                )))
            })?;
        self.add_edge(GraphEdge {
            from: family_node.clone(),
            to: skill_node,
            kind: EdgeKind::Classifies,
        });
        Ok(())
    }

    fn derive_intent_packets(
        &mut self,
        family_node: &NodeId,
        family_id: &str,
        matrix_path: &GraphPath,
        skill_ids: &[String],
        route: &str,
        limit: usize,
        dependencies: &[String],
        owned_kind: &str,
        owned_components: &mut BTreeSet<String>,
    ) -> Result<(), GraphError> {
        (limit == 0).then_some(()).map_or(Ok(()), |_| {
            Err(GraphError::InvalidValue(format!(
                "intent family `{family_id}` has a zero packet limit"
            )))
        })?;
        let family_key = family_id.strip_prefix("IF/").unwrap_or(family_id);
        skill_ids
            .chunks(limit)
            .enumerate()
            .try_for_each(|(offset, chunk)| {
                self.derive_intent_packet(
                    family_node,
                    family_id,
                    family_key,
                    matrix_path,
                    chunk,
                    route,
                    limit,
                    dependencies,
                    owned_kind,
                    offset,
                    owned_components,
                )
            })
    }

    fn derive_intent_packet(
        &mut self,
        family_node: &NodeId,
        family_id: &str,
        family_key: &str,
        matrix_path: &GraphPath,
        chunk: &[String],
        route: &str,
        limit: usize,
        dependencies: &[String],
        owned_kind: &str,
        offset: usize,
        owned_components: &mut BTreeSet<String>,
    ) -> Result<(), GraphError> {
        let packet_id = NodeId::new(format!("WP/{route}/IF-{family_key}/B{:02}", offset + 1))?;
        let gate_id = NodeId::new(format!("TEST/{packet_id}/gate"))?;
        let mut packet = GraphNode::new(
            packet_id.clone(),
            NodeKind::Workpack,
            format!("{family_id} {owned_kind} packet {}", offset + 1),
            Some(matrix_path.clone()),
            CompletionContract {
                required_paths: vec![matrix_path.clone()],
                required_tests: vec![gate_id],
                ..CompletionContract::default()
            },
        );
        packet
            .metadata
            .insert("workpackClass".to_owned(), "intent-packet".to_owned());
        packet
            .metadata
            .insert("familyId".to_owned(), family_id.to_owned());
        packet.metadata.insert("route".to_owned(), route.to_owned());
        packet
            .metadata
            .insert("ownedKind".to_owned(), owned_kind.to_owned());
        packet
            .metadata
            .insert("batchLimit".to_owned(), limit.to_string());
        packet
            .metadata
            .insert("skillCount".to_owned(), chunk.len().to_string());
        packet
            .metadata
            .insert("skillIds".to_owned(), chunk.join(","));
        self.add_node(packet)?;
        self.add_edge(GraphEdge {
            from: family_node.clone(),
            to: packet_id.clone(),
            kind: EdgeKind::RoutesTo,
        });
        std::iter::once(format!("WP/{route}"))
            .chain(dependencies.iter().cloned())
            .try_for_each(|dependency| {
                self.add_edge(GraphEdge {
                    from: packet_id.clone(),
                    to: NodeId::new(dependency)?,
                    kind: EdgeKind::DependsOn,
                });
                Ok::<(), GraphError>(())
            })?;
        chunk.iter().try_for_each(|skill_id| {
            self.record_component_ownership(skill_id, owned_kind, owned_components)?;
            self.add_edge(GraphEdge {
                from: packet_id.clone(),
                to: NodeId::new(format!("SKILL/{skill_id}"))?,
                kind: EdgeKind::RoutesTo,
            });
            Ok::<(), GraphError>(())
        })
    }

    fn record_component_ownership(
        &self,
        skill_id: &str,
        owned_kind: &str,
        owned_components: &mut BTreeSet<String>,
    ) -> Result<(), GraphError> {
        let generic_kind = (owned_kind != "advisory-manual").then_some(owned_kind);
        [
            ("advisory-manual", "advisory"),
            ("advisory-manual", "manual"),
        ]
        .into_iter()
        .filter_map(|(selector, kind)| (selector == owned_kind).then_some(kind))
        .chain(generic_kind)
        .try_for_each(|component_kind| {
            let key = format!("{skill_id}:{component_kind}");
            (!owned_components.insert(key))
                .then_some(())
                .map_or(Ok(()), |_| {
                    Err(GraphError::InvalidValue(format!(
                        "intent packet component ownership overlaps for `{skill_id}:{component_kind}`"
                    )))
                })
        })
    }

    fn validate_intent_partition(&self, assigned: &BTreeSet<String>) -> Result<(), GraphError> {
        let available: BTreeSet<String> = self
            .nodes
            .values()
            .filter(|node| {
                node.kind == NodeKind::Skill
                    && node.metadata.get("sourceAvailability").map(String::as_str)
                        != Some("sourceUnavailable")
            })
            .filter_map(|node| node.id.as_str().strip_prefix("SKILL/").map(str::to_owned))
            .collect();
        (assigned == &available).then_some(()).ok_or_else(|| {
            GraphError::InvalidValue(format!(
                "intent matrix partition mismatch: assigned {}, available {}",
                assigned.len(),
                available.len()
            ))
        })
    }
}
