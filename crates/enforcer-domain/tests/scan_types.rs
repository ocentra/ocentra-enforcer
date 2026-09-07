use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::scan_types::{
    CommitRef, LiteralConfidence, LiteralFileByteLimit, LiteralFileRole, LiteralFindingPath,
    LiteralLanguageId, LiteralRiskCategory, LiteralRiskScore, LiteralScanCount, LiteralScanToggle,
    LiteralStableHash, LiteralStringSyntaxProfile, LiteralSyntaxKind,
};

#[test]
fn literal_language_id_rejects_invalid_input() -> Result<(), Box<dyn std::error::Error>> {
    let error = LiteralLanguageId::try_new("Type Script")
        .err()
        .ok_or("mixed-case language id must be rejected")?;
    assert_eq!(error.path, "literalLanguageId");
    Ok(())
}

#[test]
fn commit_ref_rejects_blank_input() -> Result<(), Box<dyn std::error::Error>> {
    let error = "   "
        .parse::<CommitRef>()
        .err()
        .ok_or("blank commit ref must be rejected")?;
    assert_eq!(error.path, "scope.commitRef");
    Ok(())
}

#[test]
fn commit_ref_rejects_git_option_shaped_input() -> Result<(), Box<dyn std::error::Error>> {
    let error = "--output=outside.txt"
        .parse::<CommitRef>()
        .err()
        .ok_or("option-shaped commit ref must be rejected")?;
    assert_eq!(error.path, "scope.commitRef");
    assert_eq!(
        error.reason,
        "must not begin with `-` because Git would interpret it as an option"
    );
    Ok(())
}

#[test]
fn literal_numeric_values_preserve_validated_parts() -> Result<(), Box<dyn std::error::Error>> {
    let hash = LiteralStableHash::of_source(ValidationSource::from_text("stable"));
    assert_eq!(hash.to_string(), "3f63b56db2890a16fbd0d80afa5a93aa");

    let score =
        LiteralRiskScore::try_from(std::num::NonZeroU8::new(73).ok_or("non-zero score fixture")?)?;
    assert_eq!(u8::from(score), 73);
    let oversized = std::num::NonZeroU8::new(101).ok_or("oversized non-zero score fixture")?;
    assert_eq!(
        LiteralRiskScore::try_from(oversized)
            .as_ref()
            .err()
            .map(|error| error.path.as_str()),
        Some("literalRiskScore")
    );

    let limit = LiteralFileByteLimit::try_from_nonzero(
        std::num::NonZeroU64::new(4096).ok_or("non-zero fixture byte limit")?,
    );
    assert_eq!(u64::from(limit), 4096);
    Ok(())
}

#[test]
fn literal_closed_values_have_canonical_wire_names() {
    assert_eq!(LiteralConfidence::High.wire_name(), "high");
    assert_eq!(LiteralFileRole::CommonText.wire_name(), "common-text");
    assert_eq!(
        LiteralSyntaxKind::InterpolatedTemplate.wire_name(),
        "interpolated-template"
    );
    assert_eq!(
        LiteralRiskCategory::ProtocolHeaderOrMedia.wire_name(),
        "protocol-header-or-media"
    );
}

#[test]
fn literal_scan_types_preserve_boundary_invariants() -> Result<(), Box<dyn std::error::Error>> {
    assert!(LiteralScanToggle::from(true).is_enabled());
    assert!(!LiteralScanToggle::from(false).is_enabled());

    let path = LiteralFindingPath::try_new("crates/example/src/lib.rs".to_owned())?;
    assert_eq!(path.as_str(), "crates/example/src/lib.rs");
    for invalid in [String::new(), "crates\\example\\src\\lib.rs".to_owned()] {
        assert_eq!(
            LiteralFindingPath::try_new(invalid)
                .as_ref()
                .err()
                .map(|error| error.path.as_str()),
            Some("literalFindingPath")
        );
    }

    let syntax = LiteralStringSyntaxProfile::from_bits(
        LiteralStringSyntaxProfile::SINGLE_QUOTE | LiteralStringSyntaxProfile::BACKTICK,
    );
    assert!(syntax.supports_single_quote());
    assert!(syntax.supports_backtick());
    assert!(!syntax.supports_triple_double());

    let count = LiteralScanCount::from_count(7);
    assert_eq!(count.get(), 7);
    Ok(())
}
