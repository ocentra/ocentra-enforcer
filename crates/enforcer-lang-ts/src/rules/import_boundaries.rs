//! `typescript/import-boundaries` — TS-4.1 (import boundary policy must be
//! respected). Detects an `import ... from "..."` statement whose source
//! path crosses a forbidden layer boundary: `domain/**` importing from
//! `infrastructure/**` or `ui/**` (the two directions the doctrine calls
//! out — domain must stay free of infra/presentation concerns).
//!
//! Position guard (mem-arc-06-0002): boundary violation is a property of
//! (importING file's layer, importED path's layer) — a bare substring
//! match on the import target alone would flag a legitimate same-layer
//! import that happens to mention "infrastructure" in an unrelated
//! identifier. This validator only inspects `import`/`from` statement
//! lines and only flags cross-layer combinations against the CURRENT
//! file's own path.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::finding::{from_source, SourceFinding};
use crate::boundary::source_analysis::import_target;
use crate::boundary::source_text::lines;

const RULE_ID: &str = "TS-4.1";

/// Forbidden (importer-layer-substring, imported-path-substring) pairs.
const FORBIDDEN_PAIRS: &[(&str, &str)] = &[("/domain/", "/infrastructure/"), ("/domain/", "/ui/")];

/// `typescript/import-boundaries` validator for TS-4.1.
#[derive(Debug)]
#[doc = "TypeScript import-boundary validator."]
pub struct ImportBoundariesValidator {
    rule_id: RuleId,
}

impl ImportBoundariesValidator {
    /// Build the validator.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id(RULE_ID)?,
        })
    }
}

impl Validator for ImportBoundariesValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let importer_path = format!("/{}", input.file.as_str());
        let mut findings = Vec::new();
        for line in lines(input.source) {
            let Some(target) = import_target(line.text.as_str()) else {
                continue;
            };
            for (importer_layer, forbidden_layer) in FORBIDDEN_PAIRS {
                if importer_path.contains(importer_layer) && target.contains(forbidden_layer) {
                    findings.extend(from_source(
                        &self.rule_id,
                        input.file,
                        SourceFinding {
                            severity: Severity::Error,
                            title: "Import boundary policy must be respected",
                            detail: format!(
                                "line {}: `{importer_layer}` file imports forbidden `{forbidden_layer}` target `{target}`",
                                line.number
                            ),
                            line: line.number,
                            snippet: Some(line.text.as_str().trim()),
                        },
                    ));
                }
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::ImportBoundariesValidator;
    use crate::boundary::test_fixtures::run_fixture_parity;

    #[test]
    fn fires_on_domain_importing_infrastructure_and_stays_silent_on_layered_import(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = ImportBoundariesValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/import-boundaries/ts-4-1/domain/fail.ts",
            "fixtures/import-boundaries/ts-4-1/domain/pass.ts",
        )?;
        Ok(())
    }
}
