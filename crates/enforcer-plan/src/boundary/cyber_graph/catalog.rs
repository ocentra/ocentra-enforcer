//! BOUNDARY-INVARIANT: catalog rows are converted into graph skill metadata
//! without promoting decomposition or proof evidence into implementation.
//! NEGATIVE-TEST: malformed catalog rows remain represented as validation
//! findings instead of silently becoming complete records.
use super::json::{coverage_field, coverage_name, string_field};
use super::{
    CompletionContract, CyberPlanGraph, EdgeKind, GraphEdge, GraphError, GraphIssue, GraphNode,
    IssueLevel, NodeId, NodeKind, PROTECTED_SKILL,
};
use serde_json::Value;
use std::fs;

impl CyberPlanGraph {
    pub(crate) fn import_catalog(&mut self) -> Result<(), GraphError> {
        let path = self.root.join(self.manifest.catalog_path.as_str());
        let catalog: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
        let records = catalog
            .get("records")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                GraphError::InvalidValue("catalog records array is missing".to_owned())
            })?;
        for record in records {
            self.import_one_catalog_record(record)?;
        }
        Ok(())
    }

    fn import_one_catalog_record(&mut self, record: &Value) -> Result<(), GraphError> {
        let Some(catalog_id) = string_field(record, &["catalogId"]) else {
            self.issues.push(GraphIssue {
                level: IssueLevel::Error,
                code: "CATALOG-ID-MISSING".to_owned(),
                node: None,
                message: "catalog record has no catalogId".to_owned(),
            });
            return Ok(());
        };
        let id = NodeId::new(format!("SKILL/{catalog_id}"))?;
        let availability =
            string_field(record, &["sourceAvailability"]).unwrap_or_else(|| "unknown".to_owned());
        let decomposition = string_field(record, &["cp08Projection", "status"])
            .or_else(|| string_field(record, &["decompositionState"]))
            .unwrap_or_else(|| "unreviewed".to_owned());
        let implementation = coverage_field(record, &["implementation", "native", "coverage"]);
        let proof = coverage_field(record, &["implementation", "executableProof", "coverage"]);
        let mut node = GraphNode::new(
            id.clone(),
            NodeKind::Skill,
            catalog_id.clone(),
            None,
            CompletionContract::default(),
        );
        node.metadata
            .insert("sourceAvailability".to_owned(), availability.clone());
        node.metadata
            .insert("decomposition".to_owned(), decomposition);
        node.metadata.insert(
            "implementationCoverage".to_owned(),
            coverage_name(implementation),
        );
        node.metadata
            .insert("proofCoverage".to_owned(), coverage_name(proof));
        if availability == "sourceUnavailable" || catalog_id == PROTECTED_SKILL {
            node.metadata
                .insert("protectedBoundary".to_owned(), "excluded".to_owned());
        } else if let Some(source_path) = string_field(record, &["sourcePath"]) {
            node.metadata.insert("sourcePath".to_owned(), source_path);
        }
        self.add_node(node)?;
        self.add_edge(GraphEdge {
            from: self.manifest.plan.id.clone(),
            to: id,
            kind: EdgeKind::Contains,
        });
        Ok(())
    }
}
