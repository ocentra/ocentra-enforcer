use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::coordination_types::{
    ClaimComparisonKey, ClaimGroup, ClaimPath, ClaimReason, CoordinationBranch,
    CoordinationOwnerIdentity, CoordinationProjectId, CoordinationRepository, CoordinationWorktree,
    IterationReason, LockKind, NodeId, NodeName, OnConflict, Operation,
};
use proptest::proptest;

fn assert_rejected<T>(result: Result<T, DecodeError>, path: &str) -> Result<(), DecodeError> {
    match result {
        Err(error) => {
            assert_eq!(error.path, path);
            assert_ne!(error.reason, "");
            Ok(())
        }
        Ok(_) => Err(DecodeError::new(
            path,
            "expected invalid input to be rejected",
        )),
    }
}

fn parse_node_id(raw: &str) -> Result<NodeId, DecodeError> {
    NodeId::parse(raw.to_owned())
}

fn parse_node_name(raw: &str) -> Result<NodeName, DecodeError> {
    NodeName::parse(raw.to_owned())
}

#[test]
fn coordination_identity_types_reject_invalid_wire_values() {
    assert_eq!(
        parse_node_id("node_valid-01").map(|value| value.as_str().to_owned()),
        Ok("node_valid-01".to_owned())
    );
    assert_eq!(
        parse_node_name("Builder.Host").map(|value| value.as_str().to_owned()),
        Ok("Builder.Host".to_owned())
    );
    assert!(parse_node_id("contains spaces").is_err());
    assert!(parse_node_name("").is_err());
}

proptest! {
    #[test]
    fn node_id_parse_accepts_generated_safe_identities(raw in "[A-Za-z0-9._-]{1,96}") {
        let parsed = NodeId::parse(raw.clone());
        assert_eq!(parsed.map(|value| value.as_str().to_owned()), Ok(raw));
    }
}

#[test]
fn coordination_policy_types_decode_only_known_wire_values() -> Result<(), DecodeError> {
    assert_eq!(
        LockKind::parse("branchLease").map(LockKind::as_str),
        Ok("branchLease")
    );
    assert_eq!(
        Operation::parse("pr_ready").map(Operation::as_str),
        Ok("pr_ready")
    );
    assert_eq!(
        IterationReason::parse("generatorDeclined").map(IterationReason::as_str),
        Ok("generatorDeclined")
    );
    assert_eq!(OnConflict::parse("intent"), Ok(OnConflict::Intent));
    assert_rejected(LockKind::parse("unknown"), "lockKind")?;
    assert_rejected(Operation::parse("unknown"), "operation")?;
    assert_rejected(IterationReason::parse("unknown"), "iterationReason")?;
    assert_rejected(OnConflict::parse("unknown"), "onConflict")?;
    Ok(())
}

#[test]
fn coordination_claim_brands_preserve_valid_wire_values() {
    assert_eq!(
        ClaimPath::parse("src/**/*.rs").map(|value| value.to_string()),
        Ok("src/**/*.rs".to_owned())
    );
    assert_eq!(
        CoordinationProjectId::parse("ocentra-enforcer").map(|value| value.to_string()),
        Ok("ocentra-enforcer".to_owned())
    );
    assert_eq!(
        CoordinationRepository::parse("https://example.test/repo.git")
            .map(|value| value.to_string()),
        Ok("https://example.test/repo.git".to_owned())
    );
    assert_eq!(
        CoordinationWorktree::parse("E:/repo-worktree").map(|value| value.to_string()),
        Ok("E:/repo-worktree".to_owned())
    );
    assert_eq!(
        CoordinationBranch::parse("rust-build").map(|value| value.to_string()),
        Ok("rust-build".to_owned())
    );
    assert_eq!(
        CoordinationOwnerIdentity::parse("thread-123").map(|value| value.to_string()),
        Ok("thread-123".to_owned())
    );
    assert_eq!(
        ClaimGroup::parse("domain-recovery").map(|value| value.to_string()),
        Ok("domain-recovery".to_owned())
    );
    assert_eq!(
        ClaimReason::parse("recover typed coordination fields").map(|value| value.to_string()),
        Ok("recover typed coordination fields".to_owned())
    );
    assert_eq!(
        ClaimComparisonKey::parse("project:repo:path").map(|value| value.to_string()),
        Ok("project:repo:path".to_owned())
    );
}

#[test]
fn coordination_claim_brands_reject_blank_wire_values() -> Result<(), DecodeError> {
    assert_rejected(ClaimPath::parse("  "), "claimPath")?;
    assert_rejected(CoordinationProjectId::parse(""), "coordinationProjectId")?;
    assert_rejected(
        CoordinationRepository::parse("\t"),
        "coordinationRepository",
    )?;
    assert_rejected(CoordinationWorktree::parse("\n"), "coordinationWorktree")?;
    assert_rejected(CoordinationBranch::parse(" "), "coordinationBranch")?;
    assert_rejected(
        CoordinationOwnerIdentity::parse(" "),
        "coordinationOwnerIdentity",
    )?;
    assert_rejected(ClaimGroup::parse(" "), "claimGroup")?;
    assert_rejected(ClaimReason::parse(" "), "claimReason")?;
    assert_rejected(ClaimComparisonKey::parse(" "), "claimComparisonKey")?;
    Ok(())
}
