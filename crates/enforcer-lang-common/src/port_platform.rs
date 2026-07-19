//! `PORT-1.1` — platform-specific script commands must be guarded, scoped
//! to a project's declared `supportedPlatforms` (arc-03's
//! `EffectiveConfig::supported_platforms`). Per the workpack's declared-scope
//! relaxation: a project that declares e.g. `["linux"]` only is NOT
//! hard-failed for a Linux-only script; the rule fires ONLY on
//! platform-specific code that falls outside the project's declared scope.
//!
//! Missing/absent `supportedPlatforms` must NOT silently relax the check:
//! per the workpack, "no declaration + any platform-specific script fails
//! (unchanged default)". Because `EffectiveConfig::supported_platforms`
//! itself defaults to `Platform::all()` on the wire (its
//! `#[serde(default = "Platform::all")]`), an explicit `["windows",
//! "macos", "linux"]` declaration and a genuinely ABSENT field are
//! indistinguishable once decoded into a bare `Vec<Platform>`. This
//! validator therefore takes an explicit [`DeclaredScope`] tri-state at
//! construction — callers resolving a project's real config pass
//! `DeclaredScope::Declared` only when the project's own config source
//! actually wrote the field, and `DeclaredScope::Undeclared` (the strict,
//! unchanged legacy behavior: fire on ANY unguarded platform-specific
//! marker) when it did not, keeping the relaxation opt-in rather than a
//! side effect of `enforcer-config`'s total-struct default.

use enforcer_domain::config_types::Platform;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::source_analysis::{
    is_documentation_line, is_portability_target, platform_label, GUARD_MARKERS, PLATFORM_MARKERS,
};

/// Whether a project's `supportedPlatforms` was actually declared, and if
/// so, which platforms. Distinguishing this from a bare `Vec<Platform>` is
/// the point: `enforcer-config` defaults an absent field to all three
/// platforms for its OWN total-struct guarantee, but PORT-1.1 must keep
/// undeclared strict (fire on any unguarded platform marker) rather than
/// inherit that default as if it were an explicit `["windows", "macos",
/// "linux"]` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredScope {
    /// The project explicitly declared its supported platform set.
    Declared(Vec<Platform>),
    /// `supportedPlatforms` was absent from every config layer: unchanged
    /// strict default, every unguarded platform-specific marker fails.
    Undeclared,
}

/// `PORT-1.1`: platform-specific script commands must be guarded, or fall
/// within the project's declared `supportedPlatforms`.
#[derive(Debug)]
pub struct PortabilityValidator {
    rule_id: RuleId,
    scope: DeclaredScope,
}

impl PortabilityValidator {
    /// Build the validator scoped to a project's declared platforms (or
    /// [`DeclaredScope::Undeclared`] for the unchanged strict default).
    pub fn new(rule_id: RuleId, scope: DeclaredScope) -> Self {
        Self { rule_id, scope }
    }
}

impl Validator for PortabilityValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if !is_portability_target(input.file) {
            return Vec::new();
        }
        for (line_idx, line) in input.source.as_str().lines().enumerate() {
            if is_documentation_line(line) {
                continue;
            }
            if GUARD_MARKERS.iter().any(|guard| line.contains(guard)) {
                continue;
            }
            for pm in PLATFORM_MARKERS {
                if !pm.matches(line) {
                    continue;
                }
                let out_of_scope = match &self.scope {
                    DeclaredScope::Undeclared => true,
                    DeclaredScope::Declared(platforms) => !platforms.contains(&pm.platform),
                };
                if out_of_scope {
                    return crate::boundary::finding(
                        &self.rule_id,
                        Severity::Error,
                        (
                            "platform-specific script commands must be guarded",
                            format!(
                                "unguarded `{}`-specific marker `{}` falls outside declared supportedPlatforms",
                                platform_label(pm.platform),
                                pm.marker
                            ),
                            Some(line.trim()),
                        ),
                        input.file,
                        crate::boundary::line_number(line_idx),
                    )
                    .into_iter()
                    .collect();
                }
            }
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{DeclaredScope, PortabilityValidator};
    use enforcer_domain::config_types::Platform;
    use enforcer_domain::findings::ScanScope;
    use enforcer_validator::validator::{ValidationInput, Validator};

    fn file() -> Result<enforcer_domain::paths::RelPath, Box<dyn std::error::Error>> {
        Ok(crate::boundary::static_rel_path("scripts/build.sh")?)
    }

    #[test]
    fn linux_only_scope_passes_bash_only_script() -> Result<(), Box<dyn std::error::Error>> {
        let validator = PortabilityValidator::new(
            crate::boundary::static_rule_id("PORT-1.1")?,
            DeclaredScope::Declared(vec![Platform::Linux]),
        );
        let source = "#!/bin/sh\necho building on linux.sh\n";
        let findings = validator.validate(ValidationInput {
            file: &file()?,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn linux_only_scope_fails_unguarded_windows_script() -> Result<(), Box<dyn std::error::Error>> {
        let validator = PortabilityValidator::new(
            crate::boundary::static_rule_id("PORT-1.1")?,
            DeclaredScope::Declared(vec![Platform::Linux]),
        );
        let source = "call build.ps1 --release\n";
        let findings = validator.validate(ValidationInput {
            file: &file()?,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        Ok(())
    }

    #[test]
    fn guarded_platform_specific_line_is_silent_even_outside_scope(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = PortabilityValidator::new(
            crate::boundary::static_rule_id("PORT-1.1")?,
            DeclaredScope::Declared(vec![Platform::Linux]),
        );
        let source = "if (process.platform === 'win32') { run('build.ps1'); }\n";
        let findings = validator.validate(ValidationInput {
            file: &file()?,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn no_declaration_is_unchanged_strict_default_and_still_fails(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = PortabilityValidator::new(
            crate::boundary::static_rule_id("PORT-1.1")?,
            DeclaredScope::Undeclared,
        );
        let source = "call build.ps1\n";
        let findings = validator.validate(ValidationInput {
            file: &file()?,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1, "unguarded platform script always fails");
        Ok(())
    }

    #[test]
    fn explicit_all_three_platforms_declared_is_fully_relaxed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Distinct from `Undeclared`: an explicit declaration of every
        // platform means nothing is out of scope, so an unguarded Windows
        // script is allowed here even though the same marker fails under
        // `Undeclared` above.
        let validator = PortabilityValidator::new(
            crate::boundary::static_rule_id("PORT-1.1")?,
            DeclaredScope::Declared(Platform::all()),
        );
        let source = "call build.ps1\n";
        let findings = validator.validate(ValidationInput {
            file: &file()?,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn extension_markers_require_a_token_boundary_after_the_extension(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = PortabilityValidator::new(
            crate::boundary::static_rule_id("PORT-1.1")?,
            DeclaredScope::Undeclared,
        );
        let source =
            "digest.sha256\npolicy.should_emit\ncommand.cmdline\njob.batch\nscript.ps1xml\n";
        let findings = validator.validate(ValidationInput {
            file: &file()?,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
            scope: ScanScope::Files,
        });
        assert!(
            findings.is_empty(),
            "identifier/member substrings are not script paths"
        );
        Ok(())
    }

    #[test]
    fn complete_script_extensions_remain_forbidden() -> Result<(), Box<dyn std::error::Error>> {
        for source in [
            "run scripts/build.sh\n",
            "run scripts/build.ps1 --release\n",
            "run scripts/build.cmd /quiet\n",
            "run scripts/build.bat\n",
        ] {
            let validator = PortabilityValidator::new(
                crate::boundary::static_rule_id("PORT-1.1")?,
                DeclaredScope::Undeclared,
            );
            let findings = validator.validate(ValidationInput {
                file: &file()?,
                source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
                scope: ScanScope::Files,
            });
            assert_eq!(
                findings.len(),
                1,
                "complete extension must remain governed: {source}"
            );
        }
        Ok(())
    }

    #[test]
    fn osascript_requires_a_complete_token() -> Result<(), Box<dyn std::error::Error>> {
        let validator = PortabilityValidator::new(
            crate::boundary::static_rule_id("PORT-1.1")?,
            DeclaredScope::Undeclared,
        );
        for source in ["osascript deploy.scpt\n", "run(osascript)\n"] {
            let findings = validator.validate(ValidationInput {
                file: &file()?,
                source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
                scope: ScanScope::Files,
            });
            assert_eq!(
                findings.len(),
                1,
                "complete osascript token must remain governed"
            );
        }
        for source in ["myosascript = false\n", "osascript_runner()\n"] {
            let findings = validator.validate(ValidationInput {
                file: &file()?,
                source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
                scope: ScanScope::Files,
            });
            assert!(
                findings.is_empty(),
                "embedded osascript text is not a command token"
            );
        }
        Ok(())
    }
}
