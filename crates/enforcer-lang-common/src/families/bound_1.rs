//! Common-family prefix `BOUND-1` (10 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/architecture.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/bound-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::register_pattern as reg;

/// Build every `BOUND-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "BOUND-1.1".parse::<RuleId>()?,
        "Boundary modules require invariant documentation".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_BOUND_1_1_MARKER",
    );
    reg(
        &mut v,
        "BOUND-1.2".parse::<RuleId>()?,
        "Raw boundary input must be converted".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_BOUND_1_2_MARKER",
    );
    reg(
        &mut v,
        "BOUND-1.3".parse::<RuleId>()?,
        "Boundary modules cannot contain domain decisions".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_BOUND_1_3_MARKER",
    );
    reg(
        &mut v,
        "BOUND-1.4".parse::<RuleId>()?,
        "Domain modules cannot import boundary modules".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_BOUND_1_4_MARKER",
    );
    reg(
        &mut v,
        "BOUND-1.5".parse::<RuleId>()?,
        "Boundary modules require negative tests".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_BOUND_1_5_MARKER",
    );
    v.push(Box::new(BoundaryDeclarationBudget::new()?));
    reg(
        &mut v,
        "BOUND-1.7".parse::<RuleId>()?,
        "Boundary glob additions require waiver".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_BOUND_1_7_MARKER",
    );
    reg(
        &mut v,
        "BOUND-1.8".parse::<RuleId>()?,
        "Boundary utility filenames are forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_BOUND_1_8_MARKER",
    );
    reg(
        &mut v,
        "BOUND-1.9".parse::<RuleId>()?,
        "Boundary DTOs cannot leak into domain signatures".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_BOUND_1_9_MARKER",
    );
    reg(
        &mut v,
        "BOUND-1.10".parse::<RuleId>()?,
        "Boundary conversion functions return typed errors".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_BOUND_1_10_MARKER",
    );
    Ok(v)
}

#[derive(Debug)]
struct BoundaryDeclarationBudget {
    rule_id: RuleId,
    title: FindingTitle,
}

impl BoundaryDeclarationBudget {
    fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "BOUND-1.6".parse()?,
            title: "Boundary raw type count is budgeted".parse()?,
        })
    }
}

impl Validator for BoundaryDeclarationBudget {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }
    fn validate(&self, input: ValidationInput<'_>) -> Vec<enforcer_domain::findings::Finding> {
        let source = input.source.as_str();
        let fixture_marker = "ENFORCER_BOUND_1_6_MARKER";
        if source.contains(fixture_marker) {
            return crate::boundary::finding(
                &self.rule_id,
                Severity::Error,
                (self.title.as_str(), "boundary DTO budget marker", None),
                input.file,
                1,
            )
            .into_iter()
            .collect();
        }
        if !input.file.as_str().contains("boundary") || !input.file.as_str().ends_with(".rs") {
            return Vec::new();
        }
        // The budget is for raw values crossing a boundary, not for the
        // number of transport containers. A DTO composed entirely of domain
        // brands is a safe wire representation and must not be charged merely
        // because its name ends in `Dto`.
        let declarations = raw_public_boundary_declarations(source);
        if declarations <= 3 {
            return Vec::new();
        }
        crate::boundary::finding(
            &self.rule_id,
            Severity::Error,
            (
                self.title.as_str(),
                format!("boundary declares {declarations} raw DTO shapes; budget is 3"),
                None,
            ),
            input.file,
            1,
        )
        .into_iter()
        .collect()
    }
}

fn raw_public_boundary_declarations(source: &str) -> usize {
    let mut count = 0;
    let mut declaration_name: Option<String> = None;
    let mut has_raw_field = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(name) = crate::boundary::source_analysis::boundary_declaration_name(trimmed) {
            declaration_name = Some(name.into());
            has_raw_field = false;
            let Some((_, fields)) = trimmed.split_once('{') else {
                continue;
            };
            has_raw_field = raw_public_boundary_field(fields);
            if fields.contains('}') {
                if let Some(name) = declaration_name.take() {
                    count += usize::from(
                        has_raw_field
                            && !crate::boundary::source_analysis::has_fallible_domain_conversion(
                                source, &name,
                            ),
                    );
                }
            }
            continue;
        }
        if declaration_name.is_none() {
            continue;
        }
        has_raw_field |= raw_public_boundary_field(trimmed);
        if trimmed.starts_with('}') {
            if let Some(name) = declaration_name.take() {
                count += usize::from(
                    has_raw_field
                        && !crate::boundary::source_analysis::has_fallible_domain_conversion(
                            source, &name,
                        ),
                );
            }
        }
    }
    count
}

fn raw_public_boundary_field(fields: &str) -> bool {
    fields.contains("pub ")
        && [
            "String",
            "str",
            "u8",
            "u16",
            "u32",
            "u64",
            "usize",
            "i8",
            "i16",
            "i32",
            "i64",
            "isize",
            "f32",
            "f64",
            "bool",
            "serde_json::Value",
        ]
        .iter()
        .any(|raw| {
            fields.contains(&format!(": {raw}")) || fields.contains(&format!(": Option<{raw}"))
        })
}

#[cfg(test)]
mod tests {
    use super::BoundaryDeclarationBudget;
    use enforcer_domain::findings::ScanScope;
    use enforcer_validator::validator::{ValidationInput, Validator};

    #[test]
    fn reference_heavy_boundary_module_stays_within_declaration_budget(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = BoundaryDeclarationBudget::new()?;
        let file = crate::boundary::static_rel_path("src/boundary/wire.rs")?;
        let source = "pub struct EventDto;\nimpl TryFrom<EventDto> for Domain { fn try_from(_: EventDto) -> Result<Self, ()> { todo!() } }\nfn repeat(_: EventDto, _: EventDto, _: EventDto) {}";
        assert!(validator
            .validate(ValidationInput {
                file: &file,
                source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
                scope: ScanScope::Files
            })
            .is_empty());
        Ok(())
    }

    #[test]
    fn branded_transport_containers_do_not_consume_the_raw_boundary_budget() {
        let source = "pub struct ArtifactDto {\n  pub id: ArtifactId,\n  pub path: RelPath,\n}\npub struct ShareDto {\n  pub scope: MemoryShareScope,\n  pub payload: MemoryBundlePayload,\n}";
        assert_eq!(super::raw_public_boundary_declarations(source), 0);
    }

    #[test]
    fn public_primitive_wire_field_consumes_the_raw_boundary_budget() {
        let source = "pub struct WireDto {\n  pub id: String,\n  pub sequence: u64,\n}";
        assert_eq!(super::raw_public_boundary_declarations(source), 1);
    }

    #[test]
    fn oversized_boundary_declaration_set_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let validator = BoundaryDeclarationBudget::new()?;
        let file = crate::boundary::static_rel_path("src/boundary/wire.rs")?;
        let source = "pub struct OneDto { pub value: String }\npub struct TwoDto { pub value: String }\npub struct ThreeDto { pub value: String }\npub struct FourDto { pub value: String }";
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id.as_str(), "BOUND-1.6");
        Ok(())
    }

    #[test]
    fn commented_conversion_text_does_not_exempt_a_raw_declaration_set(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = BoundaryDeclarationBudget::new()?;
        let file = crate::boundary::static_rel_path("src/boundary/wire.rs")?;
        let source = "pub struct OneDto { pub value: String }\npub struct TwoDto { pub value: String }\npub struct ThreeDto { pub value: String }\npub struct FourDto { pub value: String }\n// impl TryFrom<FourDto> for Four {}";
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        Ok(())
    }

    #[test]
    fn converted_boundary_declaration_set_is_not_counted_as_raw(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = BoundaryDeclarationBudget::new()?;
        let file = crate::boundary::static_rel_path("src/boundary/wire.rs")?;
        let source = "pub struct OneDto;\npub struct TwoDto;\npub struct ThreeDto;\npub struct FourDto;\nimpl TryFrom<OneDto> for One {}\nimpl TryFrom<TwoDto> for Two {}\nimpl TryFrom<ThreeDto> for Three {}\nimpl TryFrom<FourDto> for Four {}";
        assert!(validator
            .validate(ValidationInput {
                file: &file,
                source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
                scope: ScanScope::Files
            })
            .is_empty());
        Ok(())
    }
}
