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

struct IntentImportContext<'a> {
    matrix_path: &'a GraphPath,
    repository_graph_skill_ids: &'a BTreeSet<String>,
    assigned: &'a mut BTreeSet<String>,
    owned_components: &'a mut BTreeSet<String>,
}

struct FamilyPacketContext<'a> {
    family_node: &'a NodeId,
    family_id: &'a str,
    matrix_path: &'a GraphPath,
    skill_ids: &'a [String],
    repository_graph_skill_ids: &'a BTreeSet<String>,
    owned_components: &'a mut BTreeSet<String>,
    family: &'a Value,
}

struct IntentPacketBatch<'a> {
    family_node: &'a NodeId,
    family_id: &'a str,
    matrix_path: &'a GraphPath,
    skill_ids: &'a [String],
    route: &'a str,
    limit: usize,
    dependencies: &'a [String],
    owned_kind: &'a str,
    owned_components: &'a mut BTreeSet<String>,
}

struct IntentPacket<'a> {
    family_node: &'a NodeId,
    family_id: &'a str,
    family_key: &'a str,
    matrix_path: &'a GraphPath,
    chunk: &'a [String],
    route: &'a str,
    limit: usize,
    dependencies: &'a [String],
    owned_kind: &'a str,
    offset: usize,
    owned_components: &'a mut BTreeSet<String>,
}

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
        let mut context = IntentImportContext {
            matrix_path: &path,
            repository_graph_skill_ids: &repository_graph_skill_ids,
            assigned: &mut assigned,
            owned_components: &mut owned_components,
        };
        families
            .iter()
            .try_for_each(|family| self.import_intent_family(family, &mut context))?;
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
        context: &mut IntentImportContext<'_>,
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
            Some(context.matrix_path.clone()),
            CompletionContract {
                required_paths: vec![context.matrix_path.clone()],
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
            self.classify_intent_skill(&family_node, skill_id, context.assigned)
        })?;
        self.derive_family_packets(&mut FamilyPacketContext {
            family_node: &family_node,
            family_id: &family_id,
            matrix_path: context.matrix_path,
            skill_ids: &skill_ids,
            repository_graph_skill_ids: context.repository_graph_skill_ids,
            owned_components: context.owned_components,
            family,
        })
    }

    fn derive_family_packets(
        &mut self,
        context: &mut FamilyPacketContext<'_>,
    ) -> Result<(), GraphError> {
        let native_route =
            string_field(context.family, &["nativeRoute"]).unwrap_or_else(|| "CP09".to_owned());
        let native_limit = usize_field(context.family, &["nativeBatchLimit"]).unwrap_or(5);
        let native_dependencies = string_array(context.family, &["dependencies"]);
        let native_skill_ids = self.native_skill_ids(context.skill_ids);
        if native_route == "CP12" {
            let (graph_skills, static_skills): (Vec<_>, Vec<_>) = native_skill_ids
                .iter()
                .cloned()
                .partition(|skill_id| context.repository_graph_skill_ids.contains(skill_id));
            self.derive_intent_packets(&mut IntentPacketBatch {
                family_node: context.family_node,
                family_id: context.family_id,
                matrix_path: context.matrix_path,
                skill_ids: &static_skills,
                route: "CP09",
                limit: 5,
                dependencies: &["WP/CP05".to_owned(), "WP/CP08".to_owned()],
                owned_kind: "native-predicate",
                owned_components: &mut *context.owned_components,
            })?;
            self.derive_intent_packets(&mut IntentPacketBatch {
                family_node: context.family_node,
                family_id: context.family_id,
                matrix_path: context.matrix_path,
                skill_ids: &graph_skills,
                route: "CP12",
                limit: native_limit,
                dependencies: &native_dependencies,
                owned_kind: "native-predicate",
                owned_components: &mut *context.owned_components,
            })?;
        } else {
            self.derive_intent_packets(&mut IntentPacketBatch {
                family_node: context.family_node,
                family_id: context.family_id,
                matrix_path: context.matrix_path,
                skill_ids: &native_skill_ids,
                route: &native_route,
                limit: native_limit,
                dependencies: &native_dependencies,
                owned_kind: "native-predicate",
                owned_components: &mut *context.owned_components,
            })?;
        }
        self.derive_intent_packets(&mut IntentPacketBatch {
            family_node: context.family_node,
            family_id: context.family_id,
            matrix_path: context.matrix_path,
            skill_ids: context.skill_ids,
            route: "CP11",
            limit: 10,
            dependencies: &["WP/CP08".to_owned()],
            owned_kind: "advisory-manual",
            owned_components: context.owned_components,
        })?;
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
        context: &mut IntentPacketBatch<'_>,
    ) -> Result<(), GraphError> {
        if context.limit == 0 {
            return Err(GraphError::InvalidValue(format!(
                "intent family `{}` has a zero packet limit",
                context.family_id
            )));
        }
        let family_key = context
            .family_id
            .strip_prefix("IF/")
            .unwrap_or(context.family_id);
        context
            .skill_ids
            .chunks(context.limit)
            .enumerate()
            .try_for_each(|(offset, chunk)| {
                let mut packet = IntentPacket {
                    family_node: context.family_node,
                    family_id: context.family_id,
                    family_key,
                    matrix_path: context.matrix_path,
                    chunk,
                    route: context.route,
                    limit: context.limit,
                    dependencies: context.dependencies,
                    owned_kind: context.owned_kind,
                    offset,
                    owned_components: &mut *context.owned_components,
                };
                self.derive_intent_packet(&mut packet)
            })
    }

    fn derive_intent_packet(&mut self, context: &mut IntentPacket<'_>) -> Result<(), GraphError> {
        let packet_id = NodeId::new(format!(
            "WP/{}/IF-{}/B{:02}",
            context.route,
            context.family_key,
            context.offset + 1
        ))?;
        let gate_id = NodeId::new(format!("TEST/{packet_id}/gate"))?;
        let mut packet = GraphNode::new(
            packet_id.clone(),
            NodeKind::Workpack,
            format!(
                "{} {} packet {}",
                context.family_id,
                context.owned_kind,
                context.offset + 1
            ),
            Some(context.matrix_path.clone()),
            CompletionContract {
                required_paths: vec![context.matrix_path.clone()],
                required_tests: vec![gate_id],
                ..CompletionContract::default()
            },
        );
        packet
            .metadata
            .insert("workpackClass".to_owned(), "intent-packet".to_owned());
        packet
            .metadata
            .insert("familyId".to_owned(), context.family_id.to_owned());
        packet
            .metadata
            .insert("route".to_owned(), context.route.to_owned());
        packet
            .metadata
            .insert("ownedKind".to_owned(), context.owned_kind.to_owned());
        packet
            .metadata
            .insert("batchLimit".to_owned(), context.limit.to_string());
        packet
            .metadata
            .insert("skillCount".to_owned(), context.chunk.len().to_string());
        packet
            .metadata
            .insert("skillIds".to_owned(), context.chunk.join(","));
        self.add_node(packet)?;
        self.add_edge(GraphEdge {
            from: context.family_node.clone(),
            to: packet_id.clone(),
            kind: EdgeKind::RoutesTo,
        });
        std::iter::once(format!("WP/{}", context.route))
            .chain(context.dependencies.iter().cloned())
            .try_for_each(|dependency| {
                self.add_edge(GraphEdge {
                    from: packet_id.clone(),
                    to: NodeId::new(dependency)?,
                    kind: EdgeKind::DependsOn,
                });
                Ok::<(), GraphError>(())
            })?;
        context.chunk.iter().try_for_each(|skill_id| {
            self.record_component_ownership(
                skill_id,
                context.owned_kind,
                context.owned_components,
            )?;
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
