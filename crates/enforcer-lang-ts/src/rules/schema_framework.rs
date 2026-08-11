//! Neutral TypeScript schema-framework recognition for UL09.
//!
//! This module emits evidence about a recognized framework shape; it never
//! decides whether that family is accepted. The doctrine profile remains the
//! policy owner, so a Zod observation can be rejected by one profile and
//! accepted by another without changing the source observation.
//!
//! BOUNDARY-INVARIANT: comments and quoted string contents are not framework
//! evidence; only import declarations and unquoted code markers are eligible.
//! NEGATIVE-TEST: tests below cover misleading comments and string literals.
//! ROUNDTRIP-TEST: crates/enforcer-lang-ts/src/rules/schema_framework.rs

use enforcer_domain::boundary::validation::{ValidationMarker, ValidationSource};
use enforcer_domain::doctrine_profile_types::DoctrineFrameworkFamily;
use enforcer_domain::telemetry_types::SourceLine;

use crate::boundary::source_text::{lines, source_line_role, SourceLineRole};

/// The normalized capability observed at one schema boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchemaCapability {
    /// A framework-provided decoder or resolver is present.
    BoundaryDecoder,
    /// A framework-provided schema/model declaration is present.
    ValidatedModel,
}

/// One framework observation before doctrine policy resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SchemaFrameworkEvidence {
    family: DoctrineFrameworkFamily,
    capability: SchemaCapability,
    line: SourceLine,
    /// BRAND-INVARIANT: this marker is selected from the adapter's closed
    /// static vocabulary and cannot be supplied by inspected source text.
    marker: ValidationMarker<'static>,
}

impl SchemaFrameworkEvidence {
    /// Return the closed framework family observed by the adapter.
    pub(crate) const fn family(self) -> DoctrineFrameworkFamily {
        self.family
    }

    /// Return the normalized capability observed by the adapter.
    pub(crate) const fn capability(self) -> SchemaCapability {
        self.capability
    }

    /// Return the validated source line carrying the observation.
    pub(crate) const fn line(self) -> SourceLine {
        self.line
    }

    /// Return the adapter-owned marker that explains the observation.
    pub(crate) const fn marker(self) -> ValidationMarker<'static> {
        self.marker
    }
}

/// Recognize the first supported schema-framework observation in source text.
pub(crate) fn recognize_schema_framework(
    source: ValidationSource<'_>,
) -> Option<SchemaFrameworkEvidence> {
    lines(source).find_map(|line| {
        if source_line_role(line.text) == SourceLineRole::CommentOnly {
            return None;
        }
        import_evidence(line.text, line.number).or_else(|| code_evidence(line.text, line.number))
    })
}

fn import_evidence(
    text: ValidationSource<'_>,
    line: SourceLine,
) -> Option<SchemaFrameworkEvidence> {
    let trimmed = text.as_str().trim_start();
    if !(trimmed.starts_with("import ") || trimmed.starts_with("export ")) {
        return None;
    }
    [
        (
            ValidationMarker::from_static("from \"zod\""),
            DoctrineFrameworkFamily::Zod,
            SchemaCapability::ValidatedModel,
        ),
        (
            ValidationMarker::from_static("from 'zod'"),
            DoctrineFrameworkFamily::Zod,
            SchemaCapability::ValidatedModel,
        ),
        (
            ValidationMarker::from_static("@effect/schema"),
            DoctrineFrameworkFamily::Effect,
            SchemaCapability::ValidatedModel,
        ),
        (
            ValidationMarker::from_static("from \"valibot\""),
            DoctrineFrameworkFamily::Valibot,
            SchemaCapability::ValidatedModel,
        ),
        (
            ValidationMarker::from_static("from 'valibot'"),
            DoctrineFrameworkFamily::Valibot,
            SchemaCapability::ValidatedModel,
        ),
    ]
    .into_iter()
    .find_map(|(marker, family, capability)| {
        trimmed
            .contains(marker.as_str())
            .then_some(SchemaFrameworkEvidence {
                family,
                capability,
                line,
                marker,
            })
    })
}

fn code_evidence(text: ValidationSource<'_>, line: SourceLine) -> Option<SchemaFrameworkEvidence> {
    [
        (
            ValidationMarker::from_static("z.object("),
            DoctrineFrameworkFamily::Zod,
            SchemaCapability::ValidatedModel,
        ),
        (
            ValidationMarker::from_static("zodResolver"),
            DoctrineFrameworkFamily::Zod,
            SchemaCapability::BoundaryDecoder,
        ),
        (
            ValidationMarker::from_static("Schema.Struct("),
            DoctrineFrameworkFamily::Effect,
            SchemaCapability::ValidatedModel,
        ),
        (
            ValidationMarker::from_static("Schema.decode"),
            DoctrineFrameworkFamily::Effect,
            SchemaCapability::BoundaryDecoder,
        ),
        (
            ValidationMarker::from_static("v.object("),
            DoctrineFrameworkFamily::Valibot,
            SchemaCapability::ValidatedModel,
        ),
        (
            ValidationMarker::from_static("v.parse("),
            DoctrineFrameworkFamily::Valibot,
            SchemaCapability::BoundaryDecoder,
        ),
    ]
    .into_iter()
    .find_map(|(marker, family, capability)| {
        contains_unquoted_code_marker(text, marker).map(|()| SchemaFrameworkEvidence {
            family,
            capability,
            line,
            marker,
        })
    })
}

fn contains_unquoted_code_marker(
    text: ValidationSource<'_>,
    marker: ValidationMarker<'static>,
) -> Option<()> {
    let bytes = text.as_str().as_bytes();
    let needle = marker.as_str().as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut index = 0;
    while index < bytes.len() {
        let &byte = bytes.get(index)?;
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == delimiter {
                quote = None;
            }
            index += 1;
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'/') {
            break;
        }
        if matches!(byte, b'\'' | b'"' | b'`') {
            quote = Some(byte);
            index += 1;
            continue;
        }
        if bytes
            .get(index..)
            .is_some_and(|suffix| suffix.starts_with(needle))
        {
            return Some(());
        }
        index += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use enforcer_domain::boundary::decode_error::DecodeError;
    use enforcer_domain::config_types::{ConfigProfileName, RuleEnabled};
    use enforcer_domain::doctrine_profile_types::{
        DoctrineFamilyPolicy, DoctrineFamilyRow, DoctrineFrameworkFamily, DoctrineLanguage,
        DoctrineProfile, DoctrineRequirement, DoctrineRequirementPolicyParts,
        DoctrineRequirementRow,
    };
    use enforcer_domain::severity::Severity;

    use super::{recognize_schema_framework, SchemaCapability, SchemaFrameworkEvidence};

    fn profile_with_zod(zod_state: RuleEnabled) -> Result<DoctrineProfile, DecodeError> {
        let requirements = DoctrineRequirement::all()
            .iter()
            .map(|requirement| {
                let families = DoctrineLanguage::Typescript
                    .valid_families()
                    .iter()
                    .map(|family| {
                        let state = if *family == DoctrineFrameworkFamily::Zod {
                            zod_state
                        } else {
                            RuleEnabled::Enabled
                        };
                        DoctrineFamilyRow::from_parts(
                            *family,
                            DoctrineFamilyPolicy::from_state(state),
                        )
                    })
                    .collect();
                DoctrineRequirementRow::try_from_parts(
                    *requirement,
                    DoctrineRequirementPolicyParts::from_parts(
                        RuleEnabled::Enabled,
                        Severity::Error,
                        families,
                        None,
                        None,
                    ),
                )
            })
            .collect::<Result<Vec<_>, DecodeError>>()?;
        DoctrineProfile::try_from_rows(
            ConfigProfileName::new("ul09-fixture".to_owned())?,
            DoctrineLanguage::Typescript,
            requirements,
            Vec::new(),
        )
    }

    fn evidence_or_error(
        source: enforcer_domain::boundary::validation::ValidationSource<'_>,
    ) -> Result<SchemaFrameworkEvidence, DecodeError> {
        recognize_schema_framework(source)
            .ok_or_else(|| DecodeError::new("evidence", "expected schema evidence"))
    }

    #[test]
    fn adapter_emits_typed_zod_evidence_from_fixture() -> Result<(), DecodeError> {
        let source = enforcer_domain::boundary::validation::ValidationSource::from_text(
            include_str!("../../tests/fixtures/frontend_react/effect-1.1/fail.ts"),
        );
        let evidence = evidence_or_error(source)?;
        assert_eq!(evidence.family(), DoctrineFrameworkFamily::Zod);
        assert_eq!(evidence.capability(), SchemaCapability::ValidatedModel);
        assert_eq!(evidence.marker().as_str(), "from \"zod\"");
        Ok(())
    }

    #[test]
    fn adapter_emits_effect_evidence_from_fixture() -> Result<(), DecodeError> {
        let source = enforcer_domain::boundary::validation::ValidationSource::from_text(
            include_str!("../../tests/fixtures/frontend_react/effect-1.1/pass.ts"),
        );
        let evidence = evidence_or_error(source)?;
        assert_eq!(evidence.family(), DoctrineFrameworkFamily::Effect);
        assert_eq!(evidence.capability(), SchemaCapability::ValidatedModel);
        Ok(())
    }

    #[test]
    fn misleading_comments_and_strings_are_not_evidence() -> Result<(), DecodeError> {
        let source = enforcer_domain::boundary::validation::ValidationSource::from_text(
            "// z.object(\nconst note = \"z.object(\";\nconst text = `zodResolver`;\n",
        );
        assert!(recognize_schema_framework(source).is_none());
        Ok(())
    }

    #[test]
    fn profile_resolution_changes_without_mutating_observation() -> Result<(), DecodeError> {
        let source = enforcer_domain::boundary::validation::ValidationSource::from_text(
            include_str!("../../tests/fixtures/frontend_react/effect-1.1/fail.ts"),
        );
        let evidence = evidence_or_error(source)?;
        assert!(profile_with_zod(RuleEnabled::Disabled)?
            .resolve(
                DoctrineLanguage::Typescript,
                DoctrineRequirement::SchemaRequired,
                evidence.family(),
            )
            .is_rejected());
        assert!(profile_with_zod(RuleEnabled::Enabled)?
            .resolve(
                DoctrineLanguage::Typescript,
                DoctrineRequirement::SchemaRequired,
                evidence.family(),
            )
            .is_accepted());
        Ok(())
    }
}
