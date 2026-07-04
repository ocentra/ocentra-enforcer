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

use enforcer_config::model::Platform;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

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

/// One platform-specific marker: the literal substring that identifies a
/// platform-specific script invocation, and which [`Platform`] it belongs
/// to.
struct PlatformMarker {
    platform: Platform,
    marker: &'static str,
}

const PLATFORM_MARKERS: &[PlatformMarker] = &[
    PlatformMarker {
        platform: Platform::Windows,
        marker: ".ps1",
    },
    PlatformMarker {
        platform: Platform::Windows,
        marker: ".cmd",
    },
    PlatformMarker {
        platform: Platform::Windows,
        marker: ".bat",
    },
    PlatformMarker {
        platform: Platform::Macos,
        marker: "osascript",
    },
    PlatformMarker {
        platform: Platform::Linux,
        marker: ".sh",
    },
];

/// A cross-platform guard marker: its presence on a line containing a
/// platform-specific marker means the script is already guarded (e.g.
/// behind an `if [ "$(uname)"` / `process.platform ===` conditional) and
/// therefore does not trip PORT-1.1 regardless of declared scope.
const GUARD_MARKERS: &[&str] = &["process.platform", "uname", "$OSTYPE", "cfg(target_os"];

/// `PORT-1.1`: platform-specific script commands must be guarded, or fall
/// within the project's declared `supportedPlatforms`.
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
        for (line_idx, line) in input.source.lines().enumerate() {
            if GUARD_MARKERS.iter().any(|guard| line.contains(guard)) {
                continue;
            }
            for pm in PLATFORM_MARKERS {
                if !line.contains(pm.marker) {
                    continue;
                }
                let out_of_scope = match &self.scope {
                    DeclaredScope::Undeclared => true,
                    DeclaredScope::Declared(platforms) => !platforms.contains(&pm.platform),
                };
                if out_of_scope {
                    return vec![Finding {
                        rule_id: self.rule_id.clone(),
                        severity: Severity::Error,
                        title: "platform-specific script commands must be guarded".to_owned(),
                        detail: format!(
                            "unguarded `{}`-specific marker `{}` falls outside declared supportedPlatforms",
                            platform_label(pm.platform),
                            pm.marker
                        ),
                        file: input.file.clone(),
                        line: (line_idx as u32).saturating_add(1),
                        snippet: Some(line.trim().to_owned()),
                    }];
                }
            }
        }
        Vec::new()
    }
}

fn platform_label(platform: Platform) -> &'static str {
    match platform {
        Platform::Windows => "windows",
        Platform::Macos => "macos",
        Platform::Linux => "linux",
    }
}

#[cfg(test)]
mod tests {
    use super::{DeclaredScope, PortabilityValidator};
    use enforcer_config::model::Platform;
    use enforcer_domain::findings::ScanScope;
    use enforcer_validator::validator::{ValidationInput, Validator};

    fn file() -> Result<enforcer_domain::paths::RelPath, Box<dyn std::error::Error>> {
        Ok("scripts/build.sh".parse()?)
    }

    #[test]
    fn linux_only_scope_passes_bash_only_script() -> Result<(), Box<dyn std::error::Error>> {
        let validator = PortabilityValidator::new(
            "PORT-1.1".parse()?,
            DeclaredScope::Declared(vec![Platform::Linux]),
        );
        let source = "#!/bin/sh\necho building on linux.sh\n";
        let findings = validator.validate(ValidationInput {
            file: &file()?,
            source,
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn linux_only_scope_fails_unguarded_windows_script() -> Result<(), Box<dyn std::error::Error>> {
        let validator = PortabilityValidator::new(
            "PORT-1.1".parse()?,
            DeclaredScope::Declared(vec![Platform::Linux]),
        );
        let source = "call build.ps1 --release\n";
        let findings = validator.validate(ValidationInput {
            file: &file()?,
            source,
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        Ok(())
    }

    #[test]
    fn guarded_platform_specific_line_is_silent_even_outside_scope(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = PortabilityValidator::new(
            "PORT-1.1".parse()?,
            DeclaredScope::Declared(vec![Platform::Linux]),
        );
        let source = "if (process.platform === 'win32') { run('build.ps1'); }\n";
        let findings = validator.validate(ValidationInput {
            file: &file()?,
            source,
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn no_declaration_is_unchanged_strict_default_and_still_fails(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = PortabilityValidator::new("PORT-1.1".parse()?, DeclaredScope::Undeclared);
        let source = "call build.ps1\n";
        let findings = validator.validate(ValidationInput {
            file: &file()?,
            source,
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
            "PORT-1.1".parse()?,
            DeclaredScope::Declared(Platform::all()),
        );
        let source = "call build.ps1\n";
        let findings = validator.validate(ValidationInput {
            file: &file()?,
            source,
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
