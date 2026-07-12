//! `CYBER-DOCKER-DAEMON.1` (T1) — harvest target: the Docker daemon
//! (`/etc/docker/daemon.json`) hardening checks named in
//! `vendor/anthropic-cybersecurity-skills/skills/hardening-docker-daemon-configuration/SKILL.md`
//! and its `scripts/agent.py` (`audit_daemon_config`, CIS Docker Benchmark
//! IDs) and `scripts/process.py` (`HARDENING_CHECKS` table). This is a
//! distinct config-file rule from the Dockerfile-instruction rule in
//! [`super::dockerfile_hardening`] — it inspects the JSON daemon
//! configuration, not a Dockerfile's build instructions.
//!
//! Vendor source maps to the following field predicates (all evaluated over
//! one parsed `daemon.json` object; a document that does not parse as a JSON
//! object is out of scope for this rule and produces no findings):
//!
//! - `insecure-registries` present and non-empty — agent.py
//!   `audit_daemon_config` (CIS 2.4, HIGH): plaintext-HTTP registries bypass
//!   image-pull authentication/TLS.
//! - `icc: true` — agent.py (CIS 2.1) / process.py `icc_disabled` (CIS 2.2,
//!   HIGH): inter-container communication left enabled on the default
//!   bridge network; hardened value is `false`.
//! - `no-new-privileges: false` (explicit) — agent.py (CIS 2.4) /
//!   process.py `no_new_privileges` (CIS 2.14, HIGH): only an explicit
//!   `false` is flagged, not an absent key, matching the vendor script's own
//!   "check the field, absence is a separate finding" split and keeping the
//!   false-positive rate low for daemon.json files that simply predate this
//!   setting.
//! - `userns-remap` set to `""` or `"none"` when present — agent.py (CIS
//!   2.8) / process.py `userns_remap` (CIS 2.9, HIGH, `expected_not_empty`).
//!   The vendor check is "not configured" (key absent); this validator
//!   narrows to the key being present but set to an empty/no-op value,
//!   which is the same "user namespace remapping disabled" misconfiguration
//!   but expressed as a hardening key present at empty. An absent key is
//!   intentionally NOT flagged here to avoid false-positiving every
//!   daemon.json that has not opted into `userns-remap`.
//! - `experimental: true` — process.py `experimental_disabled` (LOW):
//!   experimental daemon features are unsupported and may carry
//!   unstable/insecure behavior; reported as `Severity::Warning`.
//! - `live-restore: false` (explicit) — agent.py (CIS 2.2, LOW) / process.py
//!   `live_restore` (CIS 2.15, MEDIUM): without live-restore, a daemon
//!   restart/upgrade kills every running container; reported as
//!   `Severity::Warning` per the vendor's LOW/MEDIUM severity for this
//!   check (lower than the Error-level checks above).
//! - `tls: false` or `tlsverify: false` (explicit) — agent.py (CIS 2.6,
//!   HIGH) / process.py `check_tls_config`: the Docker daemon's remote API
//!   socket is unauthenticated when either flag is explicitly disabled.

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// `CYBER-DOCKER-DAEMON.1` — flags insecure Docker daemon (`daemon.json`)
/// settings: plaintext insecure registries, inter-container communication
/// left enabled, privilege-escalation not blocked, user-namespace
/// remapping disabled, experimental features enabled, live-restore
/// disabled, and an unauthenticated TLS-less remote API.
pub struct DockerDaemonHardeningValidator {
    rule_id: RuleId,
}

impl DockerDaemonHardeningValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-DOCKER-DAEMON.1".parse()?,
        })
    }

    fn finding(&self, file: &RelPath, severity: Severity, title: &str, detail: String) -> Finding {
        Finding {
            rule_id: self.rule_id.clone(),
            severity,
            title: title.to_owned(),
            detail,
            file: file.clone(),
            line: 1,
            snippet: None,
        }
    }
}

impl Validator for DockerDaemonHardeningValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(input.source) else {
            return Vec::new();
        };
        let Some(config) = value.as_object() else {
            return Vec::new();
        };

        let mut findings = Vec::new();

        if config
            .get("insecure-registries")
            .and_then(|v| v.as_array())
            .is_some_and(|registries| !registries.is_empty())
        {
            findings.push(
                self.finding(
                    input.file,
                    Severity::Error,
                    "Docker daemon allows insecure (plaintext) registries",
                    "daemon.json sets a non-empty `insecure-registries` list, which lets the \
                 daemon pull images over plaintext HTTP with no TLS/authentication. Fix: \
                 remove `insecure-registries` (or leave it empty) and configure the registry \
                 with TLS instead."
                        .to_owned(),
                ),
            );
        }

        if config.get("icc").and_then(|v| v.as_bool()) == Some(true) {
            findings.push(
                self.finding(
                    input.file,
                    Severity::Error,
                    "Docker daemon has inter-container communication enabled",
                    "daemon.json sets `icc: true`, allowing containers on the default bridge \
                 network to communicate with each other unrestricted. Fix: set `icc` to \
                 `false` and use explicit user-defined networks or published ports."
                        .to_owned(),
                ),
            );
        }

        if config.get("no-new-privileges").and_then(|v| v.as_bool()) == Some(false) {
            findings.push(
                self.finding(
                    input.file,
                    Severity::Error,
                    "Docker daemon does not block privilege escalation",
                    "daemon.json sets `no-new-privileges: false`, allowing container processes \
                 to gain additional privileges via setuid/setgid binaries or capability \
                 escalation. Fix: set `no-new-privileges` to `true`."
                        .to_owned(),
                ),
            );
        }

        if config
            .get("userns-remap")
            .and_then(|v| v.as_str())
            .is_some_and(|remap| remap.is_empty() || remap == "none")
        {
            findings.push(
                self.finding(
                    input.file,
                    Severity::Error,
                    "Docker daemon has user namespace remapping disabled",
                    "daemon.json sets `userns-remap` to an empty value or `\"none\"`, which \
                 leaves container root (UID 0) mapped directly to host root and enables a \
                 container breakout to gain root on the host. Fix: set `userns-remap` to \
                 `\"default\"` (or a configured subuid/subgid user)."
                        .to_owned(),
                ),
            );
        }

        if config.get("experimental").and_then(|v| v.as_bool()) == Some(true) {
            findings.push(
                self.finding(
                    input.file,
                    Severity::Warning,
                    "Docker daemon has experimental features enabled",
                    "daemon.json sets `experimental: true`, enabling unsupported daemon features \
                 that may carry unstable or insecure behavior. Fix: set `experimental` to \
                 `false` in production."
                        .to_owned(),
                ),
            );
        }

        if config.get("live-restore").and_then(|v| v.as_bool()) == Some(false) {
            findings.push(
                self.finding(
                    input.file,
                    Severity::Warning,
                    "Docker daemon has live-restore disabled",
                    "daemon.json sets `live-restore: false`, so every running container is \
                 killed on a daemon restart or upgrade. Fix: set `live-restore` to `true`."
                        .to_owned(),
                ),
            );
        }

        let mut disabled_tls_keys: Vec<&str> = Vec::new();
        if config.get("tls").and_then(|v| v.as_bool()) == Some(false) {
            disabled_tls_keys.push("tls");
        }
        if config.get("tlsverify").and_then(|v| v.as_bool()) == Some(false) {
            disabled_tls_keys.push("tlsverify");
        }
        if !disabled_tls_keys.is_empty() {
            findings.push(self.finding(
                input.file,
                Severity::Error,
                "Docker daemon exposes an unauthenticated remote API",
                format!(
                    "daemon.json sets `{}: false`, leaving the Docker daemon's remote API \
                     socket unauthenticated. Fix: set both `tls` and `tlsverify` to `true` \
                     and configure `tlscacert`/`tlscert`/`tlskey`.",
                    disabled_tls_keys.join("`/`")
                ),
            ));
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::DockerDaemonHardeningValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_docker_daemon() -> Result<(), Box<dyn std::error::Error>> {
        let validator = DockerDaemonHardeningValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/container.docker-daemon/bad/daemon.json",
            "tests/fixtures/cyberskills/container.docker-daemon/good/daemon.json",
        )?;
        Ok(())
    }
}
