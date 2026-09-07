//! BOUNDARY-INVARIANT: serde wire defaults are converted into the validated
//! manifest domain model before import and are not a second source of truth.
//! NEGATIVE-TEST: malformed wire values remain rejected by manifest validation.
use super::manifest::{CompletionEvidence, GraphManifest, GraphOverrides, ImportConfig, SeedNode};
use super::{GraphPath, LifecycleState, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SeedNodeWire {
    id: NodeId,
    title: String,
    path: GraphPath,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ImportConfigWire {
    #[doc = "DEFAULT-JUSTIFICATION: omitted workpack import switches retain the enabled default."]
    #[serde(default = "default_true")]
    workpacks: bool,
    #[doc = "DEFAULT-JUSTIFICATION: dependency workpack import is opt-in for v1 compatibility."]
    #[serde(default)]
    dependency_workpacks: bool,
    #[doc = "DEFAULT-JUSTIFICATION: omitted catalog import switches retain the enabled default."]
    #[serde(default = "default_true")]
    catalog: bool,
    #[doc = "DEFAULT-JUSTIFICATION: omitted CP08 import switches retain the enabled default."]
    #[serde(default = "default_true")]
    cp08_proofs: bool,
    #[doc = "DEFAULT-JUSTIFICATION: omitted CP01 import switches retain the enabled default."]
    #[serde(default = "default_true")]
    cp01_proofs: bool,
    #[doc = "DEFAULT-JUSTIFICATION: omitted intent-matrix imports preserve the v1 graph default. "]
    #[serde(default)]
    intent_matrix: bool,
    #[doc = "DEFAULT-JUSTIFICATION: omitted CP11 import switches retain the enabled default."]
    #[serde(default = "default_true")]
    cp11_proofs: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GraphOverridesWire {
    #[doc = "DEFAULT-JUSTIFICATION: absent lifecycle overrides preserve imported lifecycle values."]
    #[serde(default)]
    lifecycle: BTreeMap<NodeId, LifecycleState>,
    #[doc = "DEFAULT-JUSTIFICATION: absent dependency overrides preserve imported dependencies."]
    #[serde(default)]
    dependencies: BTreeMap<NodeId, Vec<NodeId>>,
    #[doc = "DEFAULT-JUSTIFICATION: absent gate evidence preserves the conservative non-DONE default."]
    #[serde(default)]
    evidence: BTreeMap<NodeId, CompletionEvidenceWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompletionEvidenceWire {
    run_id: String,
    command: String,
    status: String,
    exit_code: i32,
    commit: String,
    source_paths: Vec<GraphPath>,
    proves: Vec<String>,
    does_not_prove: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GraphManifestWire {
    schema_version: u32,
    graph_id: NodeId,
    goal: SeedNodeWire,
    plan: SeedNodeWire,
    workpack_index: GraphPath,
    workpack_root: GraphPath,
    #[doc = "DEFAULT-JUSTIFICATION: v1 manifests may omit the optional dependency workpack index."]
    #[serde(default)]
    dependency_workpack_index: Option<GraphPath>,
    #[doc = "DEFAULT-JUSTIFICATION: v1 manifests may omit the optional dependency workpack root."]
    #[serde(default)]
    dependency_workpack_root: Option<GraphPath>,
    test_proof_expectations: GraphPath,
    catalog_path: GraphPath,
    #[doc = "DEFAULT-JUSTIFICATION: v1 manifests may omit the optional intent matrix. "]
    #[serde(default)]
    intent_matrix_path: Option<GraphPath>,
    #[doc = "DEFAULT-JUSTIFICATION: absent proof roots mean the manifest declares no proof root."]
    #[serde(default)]
    proof_roots: Vec<GraphPath>,
    #[doc = "DEFAULT-JUSTIFICATION: absent decision roots mean the manifest declares no ADR root."]
    #[serde(default)]
    decision_roots: Vec<GraphPath>,
    #[doc = "DEFAULT-JUSTIFICATION: absent import switches use the documented default importer policy."]
    #[serde(default)]
    import: ImportConfigWire,
    #[doc = "DEFAULT-JUSTIFICATION: absent overrides mean no human-reviewed migration correction."]
    #[serde(default)]
    overrides: GraphOverridesWire,
}

impl From<SeedNodeWire> for SeedNode {
    fn from(value: SeedNodeWire) -> Self {
        Self {
            id: value.id,
            title: value.title,
            path: value.path,
        }
    }
}

impl From<ImportConfigWire> for ImportConfig {
    fn from(value: ImportConfigWire) -> Self {
        Self {
            workpacks: value.workpacks,
            dependency_workpacks: value.dependency_workpacks,
            catalog: value.catalog,
            cp08_proofs: value.cp08_proofs,
            cp01_proofs: value.cp01_proofs,
            intent_matrix: value.intent_matrix,
            cp11_proofs: value.cp11_proofs,
        }
    }
}

impl From<GraphOverridesWire> for GraphOverrides {
    fn from(value: GraphOverridesWire) -> Self {
        Self {
            lifecycle: value.lifecycle,
            dependencies: value.dependencies,
            evidence: value
                .evidence
                .into_iter()
                .map(|(node, evidence)| (node, completion_evidence_from_wire(evidence)))
                .collect(),
        }
    }
}

impl From<GraphManifestWire> for GraphManifest {
    fn from(value: GraphManifestWire) -> Self {
        Self {
            schema_version: value.schema_version,
            graph_id: value.graph_id,
            goal: value.goal.into(),
            plan: value.plan.into(),
            workpack_index: value.workpack_index,
            workpack_root: value.workpack_root,
            dependency_workpack_index: value.dependency_workpack_index,
            dependency_workpack_root: value.dependency_workpack_root,
            test_proof_expectations: value.test_proof_expectations,
            catalog_path: value.catalog_path,
            intent_matrix_path: value.intent_matrix_path,
            proof_roots: value.proof_roots,
            decision_roots: value.decision_roots,
            import: value.import.into(),
            overrides: value.overrides.into(),
        }
    }
}

impl From<&SeedNode> for SeedNodeWire {
    fn from(value: &SeedNode) -> Self {
        Self {
            id: value.id.clone(),
            title: value.title.clone(),
            path: value.path.clone(),
        }
    }
}

impl From<&ImportConfig> for ImportConfigWire {
    fn from(value: &ImportConfig) -> Self {
        Self {
            workpacks: value.workpacks,
            dependency_workpacks: value.dependency_workpacks,
            catalog: value.catalog,
            cp08_proofs: value.cp08_proofs,
            cp01_proofs: value.cp01_proofs,
            intent_matrix: value.intent_matrix,
            cp11_proofs: value.cp11_proofs,
        }
    }
}

impl From<&GraphOverrides> for GraphOverridesWire {
    fn from(value: &GraphOverrides) -> Self {
        Self {
            lifecycle: value.lifecycle.clone(),
            dependencies: value.dependencies.clone(),
            evidence: value
                .evidence
                .iter()
                .map(|(node, evidence)| (node.clone(), completion_evidence_to_wire(evidence)))
                .collect(),
        }
    }
}

impl From<&GraphManifest> for GraphManifestWire {
    fn from(value: &GraphManifest) -> Self {
        Self {
            schema_version: value.schema_version,
            graph_id: value.graph_id.clone(),
            goal: (&value.goal).into(),
            plan: (&value.plan).into(),
            workpack_index: value.workpack_index.clone(),
            workpack_root: value.workpack_root.clone(),
            dependency_workpack_index: value.dependency_workpack_index.clone(),
            dependency_workpack_root: value.dependency_workpack_root.clone(),
            test_proof_expectations: value.test_proof_expectations.clone(),
            catalog_path: value.catalog_path.clone(),
            intent_matrix_path: value.intent_matrix_path.clone(),
            proof_roots: value.proof_roots.clone(),
            decision_roots: value.decision_roots.clone(),
            import: (&value.import).into(),
            overrides: (&value.overrides).into(),
        }
    }
}

fn completion_evidence_from_wire(value: CompletionEvidenceWire) -> CompletionEvidence {
    CompletionEvidence {
        run_id: value.run_id,
        command: value.command,
        status: value.status,
        exit_code: value.exit_code,
        commit: value.commit,
        source_paths: value.source_paths,
        proves: value.proves,
        does_not_prove: value.does_not_prove,
    }
}

fn completion_evidence_to_wire(value: &CompletionEvidence) -> CompletionEvidenceWire {
    CompletionEvidenceWire {
        run_id: value.run_id.clone(),
        command: value.command.clone(),
        status: value.status.clone(),
        exit_code: value.exit_code,
        commit: value.commit.clone(),
        source_paths: value.source_paths.clone(),
        proves: value.proves.clone(),
        does_not_prove: value.does_not_prove.clone(),
    }
}
