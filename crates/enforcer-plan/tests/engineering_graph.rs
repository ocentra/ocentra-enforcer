//! BOUNDARY-INVARIANT: integration tests exercise the graph only through the graph's
//! public read-only contract and never open the vendor tree.
//! NEGATIVE-TEST: protected sourceUnavailable rows remain pathless and cannot
//! become evidence for native implementation or executable proof.

use std::error::Error;
use std::io::{Error as IoError, ErrorKind};
use std::path::PathBuf;

use enforcer_plan::graph::{CyberPlanGraph, DerivedState, NodeId, NodeKind};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const IMPORTED_CATALOG_NODES: &[(&str, NodeKind, &str)] = &[
    ("PROOF/CP11/batch-01", NodeKind::Proof, "10"),
    ("PROOF/CP11/batch-02", NodeKind::Proof, "4"),
    ("PROOF/CP11/batch-03", NodeKind::Proof, "10"),
    ("PROOF/CP11/batch-04", NodeKind::Proof, "10"),
    ("PROOF/CP11/batch-05", NodeKind::Proof, "8"),
    ("PROOF/CP11/batch-06", NodeKind::Proof, "2"),
    ("WP/CP11/IF-cloud-security/B02", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-cloud-security/B03", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-cloud-security/B04", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-cloud-security/B05", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-cloud-security/B06", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-cloud-security/B07", NodeKind::Workpack, "6"),
    (
        "WP/CP11/IF-compliance-governance/B01",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-container-security/B01",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-container-security/B02",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-container-security/B03",
        NodeKind::Workpack,
        "10",
    ),
    ("WP/CP11/IF-container-security/B04", NodeKind::Workpack, "3"),
    ("WP/CP11/IF-cryptography/B01", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-cryptography/B02", NodeKind::Workpack, "6"),
    ("WP/CP11/IF-data-protection/B01", NodeKind::Workpack, "1"),
    (
        "WP/CP11/IF-deception-technology/B01",
        NodeKind::Workpack,
        "6",
    ),
    ("WP/CP11/IF-devsecops/B01", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-devsecops/B02", NodeKind::Workpack, "8"),
    ("WP/CP11/IF-digital-forensics/B01", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-digital-forensics/B02", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-digital-forensics/B03", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-digital-forensics/B04", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-digital-forensics/B05", NodeKind::Workpack, "1"),
    ("WP/CP11/IF-endpoint-security/B01", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-endpoint-security/B02", NodeKind::Workpack, "7"),
    (
        "WP/CP11/IF-hardware-firmware-security/B01",
        NodeKind::Workpack,
        "6",
    ),
    (
        "WP/CP11/IF-identity-access-management/B01",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-identity-access-management/B02",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-identity-access-management/B03",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-identity-access-management/B04",
        NodeKind::Workpack,
        "10",
    ),
    ("WP/CP11/IF-incident-response/B01", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-incident-response/B02", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-incident-response/B03", NodeKind::Workpack, "6"),
    ("WP/CP11/IF-malware-analysis/B01", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-malware-analysis/B02", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-malware-analysis/B03", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-malware-analysis/B04", NodeKind::Workpack, "8"),
    ("WP/CP11/IF-mobile-security/B01", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-mobile-security/B02", NodeKind::Workpack, "3"),
    ("WP/CP11/IF-network-security/B01", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-network-security/B02", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-network-security/B03", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-network-security/B04", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-network-security/B05", NodeKind::Workpack, "3"),
    ("WP/CP11/IF-ot-ics-security/B01", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-ot-ics-security/B02", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-ot-ics-security/B03", NodeKind::Workpack, "9"),
    (
        "WP/CP11/IF-penetration-testing/B01",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-penetration-testing/B02",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-penetration-testing/B03",
        NodeKind::Workpack,
        "3",
    ),
    ("WP/CP11/IF-phishing-defense/B01", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-phishing-defense/B02", NodeKind::Workpack, "6"),
    ("WP/CP11/IF-privacy-compliance/B01", NodeKind::Workpack, "2"),
    ("WP/CP11/IF-purple-team/B01", NodeKind::Workpack, "1"),
    (
        "WP/CP11/IF-ransomware-defense/B01",
        NodeKind::Workpack,
        "10",
    ),
    ("WP/CP11/IF-ransomware-defense/B02", NodeKind::Workpack, "3"),
    ("WP/CP11/IF-red-teaming/B01", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-red-teaming/B02", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-red-teaming/B03", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-red-teaming/B04", NodeKind::Workpack, "5"),
    ("WP/CP11/IF-soc-operations/B01", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-soc-operations/B02", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-soc-operations/B03", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-soc-operations/B04", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-soc-operations/B05", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-soc-operations/B06", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-soc-operations/B07", NodeKind::Workpack, "3"),
    (
        "WP/CP11/IF-supply-chain-security/B01",
        NodeKind::Workpack,
        "8",
    ),
    ("WP/CP11/IF-threat-detection/B01", NodeKind::Workpack, "7"),
    ("WP/CP11/IF-threat-hunting/B01", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-threat-hunting/B02", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-threat-hunting/B03", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-threat-hunting/B04", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-threat-hunting/B05", NodeKind::Workpack, "10"),
    ("WP/CP11/IF-threat-hunting/B06", NodeKind::Workpack, "8"),
    (
        "WP/CP11/IF-threat-intelligence/B01",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-threat-intelligence/B02",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-threat-intelligence/B03",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-threat-intelligence/B04",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-threat-intelligence/B05",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-threat-intelligence/B06",
        NodeKind::Workpack,
        "2",
    ),
    (
        "WP/CP11/IF-vulnerability-management/B01",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-vulnerability-management/B02",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-vulnerability-management/B03",
        NodeKind::Workpack,
        "5",
    ),
    (
        "WP/CP11/IF-web-application-security/B01",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-web-application-security/B02",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-web-application-security/B03",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-web-application-security/B04",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-web-application-security/B05",
        NodeKind::Workpack,
        "6",
    ),
    ("WP/CP11/IF-wireless-security/B01", NodeKind::Workpack, "2"),
    (
        "WP/CP11/IF-zero-trust-architecture/B01",
        NodeKind::Workpack,
        "10",
    ),
    (
        "WP/CP11/IF-zero-trust-architecture/B02",
        NodeKind::Workpack,
        "8",
    ),
];

fn assert_imported_catalog_nodes(graph: &CyberPlanGraph) -> Result<(), Box<dyn Error>> {
    let cp01 = NodeId::new("PROOF/CP01/batch-05")?;
    let cp01_node = graph
        .node(&cp01)
        .ok_or_else(|| IoError::new(ErrorKind::NotFound, "CP01 batch-05"))?;
    assert_eq!(cp01_node.kind, NodeKind::Proof);
    assert_eq!(
        cp01_node.metadata.get("ruleCount").map(String::as_str),
        Some("4")
    );

    for &(id, kind, skill_count) in IMPORTED_CATALOG_NODES {
        let node_id = NodeId::new(id)?;
        let node = graph
            .node(&node_id)
            .ok_or_else(|| IoError::new(ErrorKind::NotFound, id))?;
        assert_eq!(node.kind, kind, "unexpected kind for {id}");
        assert_eq!(
            node.metadata.get("skillCount").map(String::as_str),
            Some(skill_count),
            "unexpected skill count for {id}"
        );
    }
    Ok(())
}

#[test]
fn imports_cyber_plan_workpacks_catalog_and_reconciliation_evidence() -> Result<(), Box<dyn Error>>
{
    let graph = CyberPlanGraph::load(repository_root())?;
    let status = graph.status();

    assert_eq!(status.workpacks.len(), 29 + status.intent.packet_count);
    assert_eq!(status.catalog.total, 817);
    assert_eq!(status.catalog.source_unavailable, 1);
    assert_eq!(status.catalog.decomposed_complete, 758);
    assert_eq!(status.catalog.decomposed_partial, 58);
    assert_eq!(status.catalog.native_complete, 6);
    assert_eq!(status.catalog.native_partial, 0);
    assert_eq!(status.catalog.proof_complete, 0);
    assert_eq!(status.intent.family_count, 34);
    assert_eq!(status.intent.mapped_skill_count, 816);
    assert_eq!(status.intent.protected_excluded, 1);
    assert!(status.intent.native_packet_count > 0);
    assert!(status.intent.retention_packet_count > 0);

    let ul00 = NodeId::new("EXT/UL00")?;
    let ul01 = NodeId::new("EXT/UL01")?;
    let ul02 = NodeId::new("EXT/UL02")?;
    let ul00_node = graph
        .node(&ul00)
        .ok_or_else(|| IoError::new(ErrorKind::NotFound, "UL00"))?;
    assert_eq!(ul00_node.kind, NodeKind::Workpack);
    assert_eq!(
        ul00_node.metadata.get("routingStatus").map(String::as_str),
        Some("READY-AUDIT")
    );
    assert_eq!(
        ul00_node.metadata.get("routingOnly").map(String::as_str),
        Some("true")
    );
    assert_eq!(graph.inspect_json(&ul00)?["state"], "done");
    assert_eq!(graph.inspect_json(&ul01)?["state"], "done");
    assert_eq!(graph.inspect_json(&ul02)?["state"], "done");
    let ul02_node = graph
        .node(&ul02)
        .ok_or_else(|| IoError::new(ErrorKind::NotFound, "UL02"))?;
    assert_eq!(ul02_node.kind, NodeKind::Workpack);
    assert_eq!(
        ul02_node.metadata.get("routingStatus").map(String::as_str),
        Some("DECISION-READY")
    );
    assert_eq!(graph.why(&ul02)?.chain, vec![ul02.clone()]);

    assert_imported_catalog_nodes(&graph)?;
    assert!(
        status.validation.is_valid(),
        "{:?}",
        status.validation.issues
    );
    Ok(())
}

fn assert_cp09_cloud_batches(graph: &CyberPlanGraph) -> Result<(), Box<dyn Error>> {
    for batch in [
        "B01", "B02", "B03", "B04", "B05", "B06", "B07", "B08", "B09", "B10", "B11", "B12",
    ] {
        let id = NodeId::new(format!("WP/CP09/IF-cloud-security/{batch}"))?;
        assert_eq!(graph.inspect(&id)?.state, DerivedState::Blocked, "{batch}");
    }
    assert!(graph
        .node(&NodeId::new("WP/CP09/IF-cloud-security/B13")?)
        .is_none());
    Ok(())
}

#[test]
fn next_selects_cp09_after_cp05_closure_without_promoting_truth() -> Result<(), Box<dyn Error>> {
    let graph = CyberPlanGraph::load(repository_root())?;
    let next = graph.next_json()?;
    for id in [
        "EXT/UL03", "EXT/UL04", "EXT/UL05", "EXT/UL06", "EXT/UL07", "EXT/UL08", "EXT/UL09",
        "EXT/UL10", "EXT/UL13",
    ] {
        assert_eq!(
            graph.inspect(&NodeId::new(id)?)?.state,
            DerivedState::Done,
            "{id} must be closed"
        );
    }
    for id in ["WP/CP02", "WP/CP03", "WP/CP04", "WP/CP05"] {
        assert_eq!(
            graph.inspect(&NodeId::new(id)?)?.state,
            DerivedState::Done,
            "{id} must be closed"
        );
    }
    let cp12 = graph.inspect(&NodeId::new("WP/CP12")?)?;
    assert_eq!(cp12.state, DerivedState::Blocked);
    assert!(cp12
        .reasons
        .iter()
        .any(|reason| reason.contains("authoritative routing status")));

    assert_eq!(next["decision"], "blocked");
    assert!(next["selected"].is_null());
    assert_cp09_cloud_batches(&graph)?;
    assert_eq!(
        graph
            .inspect(&NodeId::new("WP/CP09/IF-compliance-governance/B01")?)?
            .state,
        DerivedState::Blocked,
        "the B01 packet must inherit the authoritative CP09 block"
    );
    assert_eq!(
        graph
            .inspect(&NodeId::new("WP/CP09/IF-compliance-governance/B02")?)?
            .state,
        DerivedState::Blocked,
        "the B02 packet must inherit the authoritative CP09 block"
    );
    let cp09_cloud_node = graph
        .node(&NodeId::new("WP/CP09/IF-cloud-security/B01")?)
        .ok_or_else(|| IoError::new(ErrorKind::NotFound, "CP09 cloud-security B01"))?;
    assert_eq!(
        cp09_cloud_node.metadata.get("route").map(String::as_str),
        Some("CP09")
    );
    assert!(graph
        .node(&NodeId::new("WP/CP12/IF-cloud-security/B01")?)
        .is_none());
    assert_eq!(next["validation"]["valid"], true);
    assert_eq!(next["policy"]["decompositionPromotesImplementation"], false);
    assert_eq!(next["policy"]["decompositionPromotesProof"], false);
    Ok(())
}

#[test]
fn authoritative_routing_status_blocks_pending_workpacks() -> Result<(), Box<dyn Error>> {
    let graph = CyberPlanGraph::load(repository_root())?;
    for id in [
        "WP/CP06",
        "WP/CP07",
        "WP/CP09",
        "WP/CP10",
        "WP/CP09/IF-container-security/B01",
    ] {
        assert_eq!(
            graph.inspect(&NodeId::new(id)?)?.state,
            DerivedState::Blocked,
            "{id} must remain blocked by the authoritative plan status"
        );
    }
    assert_eq!(
        graph.inspect(&NodeId::new("WP/CP00")?)?.state,
        DerivedState::Validation
    );
    Ok(())
}

#[test]
fn protected_catalog_row_is_explicitly_excluded() -> Result<(), Box<dyn Error>> {
    let graph = CyberPlanGraph::load(repository_root())?;
    let id = NodeId::new("SKILL/detecting-fileless-malware-techniques")?;
    let node = graph
        .node(&id)
        .ok_or_else(|| IoError::new(ErrorKind::NotFound, "protected row"))?;

    assert_eq!(node.kind, NodeKind::Skill);
    assert_eq!(
        node.metadata.get("sourceAvailability").map(String::as_str),
        Some("sourceUnavailable")
    );
    assert_eq!(
        node.metadata.get("protectedBoundary").map(String::as_str),
        Some("excluded")
    );
    assert!(node.path.is_none());
    Ok(())
}

#[test]
fn native_packets_exclude_cp08_external_only_skills() -> Result<(), Box<dyn Error>> {
    let graph = CyberPlanGraph::load(repository_root())?;
    let packet = graph
        .node(&NodeId::new("WP/CP09/IF-container-security/B01")?)
        .ok_or_else(|| IoError::new(ErrorKind::NotFound, "container-security native packet"))?;

    assert_eq!(
        packet.metadata.get("skillIds").map(String::as_str),
        Some(concat!(
            "analyzing-kubernetes-audit-logs,auditing-kubernetes-rbac-privilege-escalation,",
            "detecting-container-drift-at-runtime,detecting-container-escape-attempts,",
            "detecting-privilege-escalation-in-kubernetes-pods"
        ))
    );
    assert_eq!(
        packet.metadata.get("skillCount").map(String::as_str),
        Some("5")
    );
    assert_eq!(
        packet.metadata.get("ownedKind").map(String::as_str),
        Some("native-predicate")
    );
    assert!(!packet
        .metadata
        .get("skillIds")
        .is_some_and(|ids| ids.contains("benchmarking-kubernetes-with-kube-bench")));
    Ok(())
}
