//! BOUNDARY-INVARIANT: integration tests exercise the graph only through its
//! public read-only contract and never open the vendor tree.
//! NEGATIVE-TEST: protected sourceUnavailable rows remain pathless and cannot
//! become evidence for native implementation or executable proof.

use std::error::Error;
use std::path::PathBuf;

use enforcer_plan::graph::{CyberPlanGraph, NodeId, NodeKind};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn imports_cyber_plan_workpacks_catalog_and_reconciliation_evidence() -> Result<(), Box<dyn Error>>
{
    let graph = CyberPlanGraph::load(repository_root())?;
    let status = graph.status();
    let cp01_batch05 = NodeId::new("PROOF/CP01/batch-05")?;
    let cp11_batch01 = NodeId::new("PROOF/CP11/batch-01")?;
    let cp11_batch02 = NodeId::new("PROOF/CP11/batch-02")?;
    let cp11_batch03 = NodeId::new("PROOF/CP11/batch-03")?;
    let cp11_batch04 = NodeId::new("PROOF/CP11/batch-04")?;
    let cp11_batch05 = NodeId::new("PROOF/CP11/batch-05")?;
    let cp11_batch06 = NodeId::new("PROOF/CP11/batch-06")?;
    let cp11_cloud_batch02 = NodeId::new("WP/CP11/IF-cloud-security/B02")?;
    let cp11_cloud_batch03 = NodeId::new("WP/CP11/IF-cloud-security/B03")?;
    let cp11_cloud_batch04 = NodeId::new("WP/CP11/IF-cloud-security/B04")?;
    let cp11_cloud_batch05 = NodeId::new("WP/CP11/IF-cloud-security/B05")?;
    let cp11_cloud_batch06 = NodeId::new("WP/CP11/IF-cloud-security/B06")?;
    let cp11_cloud_batch07 = NodeId::new("WP/CP11/IF-cloud-security/B07")?;
    let cp11_compliance_batch01 = NodeId::new("WP/CP11/IF-compliance-governance/B01")?;
    let cp11_container_batch01 = NodeId::new("WP/CP11/IF-container-security/B01")?;
    let cp11_container_batch02 = NodeId::new("WP/CP11/IF-container-security/B02")?;
    let cp11_container_batch03 = NodeId::new("WP/CP11/IF-container-security/B03")?;
    let cp11_container_batch04 = NodeId::new("WP/CP11/IF-container-security/B04")?;
    let cp11_crypto_batch01 = NodeId::new("WP/CP11/IF-cryptography/B01")?;
    let cp11_crypto_batch02 = NodeId::new("WP/CP11/IF-cryptography/B02")?;
    let cp11_data_protection_batch01 = NodeId::new("WP/CP11/IF-data-protection/B01")?;
    let cp11_deception_batch01 = NodeId::new("WP/CP11/IF-deception-technology/B01")?;
    let cp11_devsecops_batch01 = NodeId::new("WP/CP11/IF-devsecops/B01")?;
    let cp11_devsecops_batch02 = NodeId::new("WP/CP11/IF-devsecops/B02")?;
    let cp11_digital_forensics_batch01 = NodeId::new("WP/CP11/IF-digital-forensics/B01")?;
    let cp11_digital_forensics_batch02 = NodeId::new("WP/CP11/IF-digital-forensics/B02")?;
    let cp11_digital_forensics_batch03 = NodeId::new("WP/CP11/IF-digital-forensics/B03")?;
    let cp11_digital_forensics_batch04 = NodeId::new("WP/CP11/IF-digital-forensics/B04")?;
    let cp11_digital_forensics_batch05 = NodeId::new("WP/CP11/IF-digital-forensics/B05")?;
    let cp11_endpoint_security_batch01 = NodeId::new("WP/CP11/IF-endpoint-security/B01")?;
    let cp11_endpoint_security_batch02 = NodeId::new("WP/CP11/IF-endpoint-security/B02")?;
    let cp11_hardware_firmware_batch01 = NodeId::new("WP/CP11/IF-hardware-firmware-security/B01")?;
    let cp11_identity_access_batch01 = NodeId::new("WP/CP11/IF-identity-access-management/B01")?;
    let cp11_identity_access_batch02 = NodeId::new("WP/CP11/IF-identity-access-management/B02")?;
    let cp11_identity_access_batch03 = NodeId::new("WP/CP11/IF-identity-access-management/B03")?;
    let cp11_identity_access_batch04 = NodeId::new("WP/CP11/IF-identity-access-management/B04")?;
    let cp11_incident_response_batch01 = NodeId::new("WP/CP11/IF-incident-response/B01")?;
    let cp11_incident_response_batch02 = NodeId::new("WP/CP11/IF-incident-response/B02")?;
    let cp11_incident_response_batch03 = NodeId::new("WP/CP11/IF-incident-response/B03")?;
    let cp11_malware_analysis_batch01 = NodeId::new("WP/CP11/IF-malware-analysis/B01")?;
    let cp11_malware_analysis_batch02 = NodeId::new("WP/CP11/IF-malware-analysis/B02")?;
    let cp11_malware_analysis_batch03 = NodeId::new("WP/CP11/IF-malware-analysis/B03")?;
    let cp11_malware_analysis_batch04 = NodeId::new("WP/CP11/IF-malware-analysis/B04")?;
    let cp11_mobile_security_batch01 = NodeId::new("WP/CP11/IF-mobile-security/B01")?;
    let cp11_mobile_security_batch02 = NodeId::new("WP/CP11/IF-mobile-security/B02")?;
    let cp11_network_security_batch01 = NodeId::new("WP/CP11/IF-network-security/B01")?;
    let cp11_network_security_batch02 = NodeId::new("WP/CP11/IF-network-security/B02")?;
    let cp11_network_security_batch03 = NodeId::new("WP/CP11/IF-network-security/B03")?;
    let cp11_network_security_batch04 = NodeId::new("WP/CP11/IF-network-security/B04")?;
    let cp11_network_security_batch05 = NodeId::new("WP/CP11/IF-network-security/B05")?;
    let cp11_ot_ics_security_batch01 = NodeId::new("WP/CP11/IF-ot-ics-security/B01")?;
    let cp11_ot_ics_security_batch02 = NodeId::new("WP/CP11/IF-ot-ics-security/B02")?;
    let cp11_ot_ics_security_batch03 = NodeId::new("WP/CP11/IF-ot-ics-security/B03")?;
    let cp11_penetration_testing_batch01 = NodeId::new("WP/CP11/IF-penetration-testing/B01")?;
    let cp11_penetration_testing_batch02 = NodeId::new("WP/CP11/IF-penetration-testing/B02")?;
    let cp11_penetration_testing_batch03 = NodeId::new("WP/CP11/IF-penetration-testing/B03")?;

    assert_eq!(status.workpacks.len(), 14 + status.intent.packet_count);
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
    let cp01_node = graph
        .node(&cp01_batch05)
        .ok_or("CP01 batch-05 evidence node must be imported")?;
    assert_eq!(cp01_node.kind, NodeKind::Proof);
    assert_eq!(
        cp01_node.metadata.get("ruleCount").map(String::as_str),
        Some("4")
    );
    let cp11_node = graph
        .node(&cp11_batch01)
        .ok_or("CP11 batch-01 evidence node must be imported")?;
    assert_eq!(cp11_node.kind, NodeKind::Proof);
    assert_eq!(
        cp11_node.metadata.get("skillCount").map(String::as_str),
        Some("10")
    );
    let cp11_batch02_node = graph
        .node(&cp11_batch02)
        .ok_or("CP11 batch-02 evidence node must be imported")?;
    assert_eq!(cp11_batch02_node.kind, NodeKind::Proof);
    assert_eq!(
        cp11_batch02_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("4")
    );
    let cp11_batch03_node = graph
        .node(&cp11_batch03)
        .ok_or("CP11 batch-03 evidence node must be imported")?;
    assert_eq!(cp11_batch03_node.kind, NodeKind::Proof);
    assert_eq!(
        cp11_batch03_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_batch04_node = graph
        .node(&cp11_batch04)
        .ok_or("CP11 batch-04 evidence node must be imported")?;
    assert_eq!(cp11_batch04_node.kind, NodeKind::Proof);
    assert_eq!(
        cp11_batch04_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_batch05_node = graph
        .node(&cp11_batch05)
        .ok_or("CP11 batch-05 evidence node must be imported")?;
    assert_eq!(cp11_batch05_node.kind, NodeKind::Proof);
    assert_eq!(
        cp11_batch05_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("8")
    );
    let cp11_batch06_node = graph
        .node(&cp11_batch06)
        .ok_or("CP11 batch-06 evidence node must be imported")?;
    assert_eq!(cp11_batch06_node.kind, NodeKind::Proof);
    assert_eq!(
        cp11_batch06_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("2")
    );
    let cp11_cloud_batch02_node = graph
        .node(&cp11_cloud_batch02)
        .ok_or("CP11 cloud-security B02 packet must be imported")?;
    assert_eq!(cp11_cloud_batch02_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_cloud_batch02_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_cloud_batch03_node = graph
        .node(&cp11_cloud_batch03)
        .ok_or("CP11 cloud-security B03 packet must be imported")?;
    assert_eq!(cp11_cloud_batch03_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_cloud_batch03_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_cloud_batch04_node = graph
        .node(&cp11_cloud_batch04)
        .ok_or("CP11 cloud-security B04 packet must be imported")?;
    assert_eq!(cp11_cloud_batch04_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_cloud_batch04_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_cloud_batch05_node = graph
        .node(&cp11_cloud_batch05)
        .ok_or("CP11 cloud-security B05 packet must be imported")?;
    assert_eq!(cp11_cloud_batch05_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_cloud_batch05_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_cloud_batch06_node = graph
        .node(&cp11_cloud_batch06)
        .ok_or("CP11 cloud-security B06 packet must be imported")?;
    assert_eq!(cp11_cloud_batch06_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_cloud_batch06_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_cloud_batch07_node = graph
        .node(&cp11_cloud_batch07)
        .ok_or("CP11 cloud-security B07 packet must be imported")?;
    assert_eq!(cp11_cloud_batch07_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_cloud_batch07_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("6")
    );
    let cp11_compliance_batch01_node = graph
        .node(&cp11_compliance_batch01)
        .ok_or("CP11 compliance-governance B01 packet must be imported")?;
    assert_eq!(cp11_compliance_batch01_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_compliance_batch01_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_container_batch01_node = graph
        .node(&cp11_container_batch01)
        .ok_or("CP11 container-security B01 packet must be imported")?;
    assert_eq!(cp11_container_batch01_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_container_batch01_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_container_batch02_node = graph
        .node(&cp11_container_batch02)
        .ok_or("CP11 container-security B02 packet must be imported")?;
    assert_eq!(cp11_container_batch02_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_container_batch02_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_container_batch03_node = graph
        .node(&cp11_container_batch03)
        .ok_or("CP11 container-security B03 packet must be imported")?;
    assert_eq!(cp11_container_batch03_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_container_batch03_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_container_batch04_node = graph
        .node(&cp11_container_batch04)
        .ok_or("CP11 container-security B04 packet must be imported")?;
    assert_eq!(cp11_container_batch04_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_container_batch04_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("3")
    );
    let cp11_crypto_batch01_node = graph
        .node(&cp11_crypto_batch01)
        .ok_or("CP11 cryptography B01 packet must be imported")?;
    assert_eq!(cp11_crypto_batch01_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_crypto_batch01_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_crypto_batch02_node = graph
        .node(&cp11_crypto_batch02)
        .ok_or("CP11 cryptography B02 packet must be imported")?;
    assert_eq!(cp11_crypto_batch02_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_crypto_batch02_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("6")
    );
    let cp11_data_protection_batch01_node = graph
        .node(&cp11_data_protection_batch01)
        .ok_or("CP11 data-protection B01 packet must be imported")?;
    assert_eq!(cp11_data_protection_batch01_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_data_protection_batch01_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("1")
    );
    let cp11_deception_batch01_node = graph
        .node(&cp11_deception_batch01)
        .ok_or("CP11 deception-technology B01 packet must be imported")?;
    assert_eq!(cp11_deception_batch01_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_deception_batch01_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("6")
    );
    let cp11_devsecops_batch01_node = graph
        .node(&cp11_devsecops_batch01)
        .ok_or("CP11 DevSecOps B01 packet must be imported")?;
    assert_eq!(cp11_devsecops_batch01_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_devsecops_batch01_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_devsecops_batch02_node = graph
        .node(&cp11_devsecops_batch02)
        .ok_or("CP11 DevSecOps B02 packet must be imported")?;
    assert_eq!(cp11_devsecops_batch02_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_devsecops_batch02_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("8")
    );
    let cp11_digital_forensics_batch01_node = graph
        .node(&cp11_digital_forensics_batch01)
        .ok_or("CP11 digital-forensics B01 packet must be imported")?;
    assert_eq!(cp11_digital_forensics_batch01_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_digital_forensics_batch01_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_digital_forensics_batch02_node = graph
        .node(&cp11_digital_forensics_batch02)
        .ok_or("CP11 digital-forensics B02 packet must be imported")?;
    assert_eq!(cp11_digital_forensics_batch02_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_digital_forensics_batch02_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_digital_forensics_batch03_node = graph
        .node(&cp11_digital_forensics_batch03)
        .ok_or("CP11 digital-forensics B03 packet must be imported")?;
    assert_eq!(cp11_digital_forensics_batch03_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_digital_forensics_batch03_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_digital_forensics_batch04_node = graph
        .node(&cp11_digital_forensics_batch04)
        .ok_or("CP11 digital-forensics B04 packet must be imported")?;
    assert_eq!(cp11_digital_forensics_batch04_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_digital_forensics_batch04_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_digital_forensics_batch05_node = graph
        .node(&cp11_digital_forensics_batch05)
        .ok_or("CP11 digital-forensics B05 packet must be imported")?;
    assert_eq!(cp11_digital_forensics_batch05_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_digital_forensics_batch05_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("1")
    );
    let cp11_endpoint_security_batch01_node = graph
        .node(&cp11_endpoint_security_batch01)
        .ok_or("CP11 endpoint-security B01 packet must be imported")?;
    assert_eq!(cp11_endpoint_security_batch01_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_endpoint_security_batch01_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_endpoint_security_batch02_node = graph
        .node(&cp11_endpoint_security_batch02)
        .ok_or("CP11 endpoint-security B02 packet must be imported")?;
    assert_eq!(cp11_endpoint_security_batch02_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_endpoint_security_batch02_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("7")
    );
    let cp11_hardware_firmware_batch01_node = graph
        .node(&cp11_hardware_firmware_batch01)
        .ok_or("CP11 hardware-firmware-security B01 packet must be imported")?;
    assert_eq!(cp11_hardware_firmware_batch01_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_hardware_firmware_batch01_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("6")
    );
    let cp11_identity_access_batch01_node = graph
        .node(&cp11_identity_access_batch01)
        .ok_or("CP11 identity-access-management B01 packet must be imported")?;
    assert_eq!(cp11_identity_access_batch01_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_identity_access_batch01_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_identity_access_batch02_node = graph
        .node(&cp11_identity_access_batch02)
        .ok_or("CP11 identity-access-management B02 packet must be imported")?;
    assert_eq!(cp11_identity_access_batch02_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_identity_access_batch02_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_identity_access_batch03_node = graph
        .node(&cp11_identity_access_batch03)
        .ok_or("CP11 identity-access-management B03 packet must be imported")?;
    assert_eq!(cp11_identity_access_batch03_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_identity_access_batch03_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_identity_access_batch04_node = graph
        .node(&cp11_identity_access_batch04)
        .ok_or("CP11 identity-access-management B04 packet must be imported")?;
    assert_eq!(cp11_identity_access_batch04_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_identity_access_batch04_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_incident_response_batch01_node = graph
        .node(&cp11_incident_response_batch01)
        .ok_or("CP11 incident-response B01 packet must be imported")?;
    assert_eq!(cp11_incident_response_batch01_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_incident_response_batch01_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_incident_response_batch02_node = graph
        .node(&cp11_incident_response_batch02)
        .ok_or("CP11 incident-response B02 packet must be imported")?;
    assert_eq!(cp11_incident_response_batch02_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_incident_response_batch02_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_incident_response_batch03_node = graph
        .node(&cp11_incident_response_batch03)
        .ok_or("CP11 incident-response B03 packet must be imported")?;
    assert_eq!(cp11_incident_response_batch03_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_incident_response_batch03_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("6")
    );
    let cp11_malware_analysis_batch01_node = graph
        .node(&cp11_malware_analysis_batch01)
        .ok_or("CP11 malware-analysis B01 packet must be imported")?;
    assert_eq!(cp11_malware_analysis_batch01_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_malware_analysis_batch01_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_malware_analysis_batch02_node = graph
        .node(&cp11_malware_analysis_batch02)
        .ok_or("CP11 malware-analysis B02 packet must be imported")?;
    assert_eq!(cp11_malware_analysis_batch02_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_malware_analysis_batch02_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_malware_analysis_batch03_node = graph
        .node(&cp11_malware_analysis_batch03)
        .ok_or("CP11 malware-analysis B03 packet must be imported")?;
    assert_eq!(cp11_malware_analysis_batch03_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_malware_analysis_batch03_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_malware_analysis_batch04_node = graph
        .node(&cp11_malware_analysis_batch04)
        .ok_or("CP11 malware-analysis B04 packet must be imported")?;
    assert_eq!(cp11_malware_analysis_batch04_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_malware_analysis_batch04_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("8")
    );
    let cp11_mobile_security_batch01_node = graph
        .node(&cp11_mobile_security_batch01)
        .ok_or("CP11 mobile-security B01 packet must be imported")?;
    assert_eq!(cp11_mobile_security_batch01_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_mobile_security_batch01_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_mobile_security_batch02_node = graph
        .node(&cp11_mobile_security_batch02)
        .ok_or("CP11 mobile-security B02 packet must be imported")?;
    assert_eq!(cp11_mobile_security_batch02_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_mobile_security_batch02_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("3")
    );
    let cp11_network_security_batch01_node = graph
        .node(&cp11_network_security_batch01)
        .ok_or("CP11 network-security B01 packet must be imported")?;
    assert_eq!(cp11_network_security_batch01_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_network_security_batch01_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_network_security_batch02_node = graph
        .node(&cp11_network_security_batch02)
        .ok_or("CP11 network-security B02 packet must be imported")?;
    assert_eq!(cp11_network_security_batch02_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_network_security_batch02_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_network_security_batch03_node = graph
        .node(&cp11_network_security_batch03)
        .ok_or("CP11 network-security B03 packet must be imported")?;
    assert_eq!(cp11_network_security_batch03_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_network_security_batch03_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_network_security_batch04_node = graph
        .node(&cp11_network_security_batch04)
        .ok_or("CP11 network-security B04 packet must be imported")?;
    assert_eq!(cp11_network_security_batch04_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_network_security_batch04_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_network_security_batch05_node = graph
        .node(&cp11_network_security_batch05)
        .ok_or("CP11 network-security B05 packet must be imported")?;
    assert_eq!(cp11_network_security_batch05_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_network_security_batch05_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("3")
    );
    let cp11_ot_ics_security_batch01_node = graph
        .node(&cp11_ot_ics_security_batch01)
        .ok_or("CP11 OT/ICS security B01 packet must be imported")?;
    assert_eq!(cp11_ot_ics_security_batch01_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_ot_ics_security_batch01_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_ot_ics_security_batch02_node = graph
        .node(&cp11_ot_ics_security_batch02)
        .ok_or("CP11 OT/ICS security B02 packet must be imported")?;
    assert_eq!(cp11_ot_ics_security_batch02_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_ot_ics_security_batch02_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_ot_ics_security_batch03_node = graph
        .node(&cp11_ot_ics_security_batch03)
        .ok_or("CP11 OT/ICS security B03 packet must be imported")?;
    assert_eq!(cp11_ot_ics_security_batch03_node.kind, NodeKind::Workpack);
    assert_eq!(
        cp11_ot_ics_security_batch03_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("9")
    );
    let cp11_penetration_testing_batch01_node = graph
        .node(&cp11_penetration_testing_batch01)
        .ok_or("CP11 penetration-testing B01 packet must be imported")?;
    assert_eq!(
        cp11_penetration_testing_batch01_node.kind,
        NodeKind::Workpack
    );
    assert_eq!(
        cp11_penetration_testing_batch01_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_penetration_testing_batch02_node = graph
        .node(&cp11_penetration_testing_batch02)
        .ok_or("CP11 penetration-testing B02 packet must be imported")?;
    assert_eq!(
        cp11_penetration_testing_batch02_node.kind,
        NodeKind::Workpack
    );
    assert_eq!(
        cp11_penetration_testing_batch02_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("10")
    );
    let cp11_penetration_testing_batch03_node = graph
        .node(&cp11_penetration_testing_batch03)
        .ok_or("CP11 penetration-testing B03 packet must be imported")?;
    assert_eq!(
        cp11_penetration_testing_batch03_node.kind,
        NodeKind::Workpack
    );
    assert_eq!(
        cp11_penetration_testing_batch03_node
            .metadata
            .get("skillCount")
            .map(String::as_str),
        Some("3")
    );
    assert!(
        status.validation.is_valid(),
        "{:?}",
        status.validation.issues
    );
    Ok(())
}

#[test]
fn next_selects_the_first_dependency_legal_packet_without_promoting_truth(
) -> Result<(), Box<dyn Error>> {
    let graph = CyberPlanGraph::load(repository_root())?;
    let next = graph.next_json()?;

    assert_eq!(next["decision"], "selected");
    assert_eq!(next["selected"]["id"], "WP/CP11/IF-phishing-defense/B01");
    assert_eq!(next["validation"]["valid"], true);
    assert_eq!(next["policy"]["decompositionPromotesImplementation"], false);
    assert_eq!(next["policy"]["decompositionPromotesProof"], false);
    Ok(())
}

#[test]
fn protected_catalog_row_is_explicitly_excluded() -> Result<(), Box<dyn Error>> {
    let graph = CyberPlanGraph::load(repository_root())?;
    let id = NodeId::new("SKILL/detecting-fileless-malware-techniques")?;
    let node = graph.node(&id).ok_or("protected row must be represented")?;

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
