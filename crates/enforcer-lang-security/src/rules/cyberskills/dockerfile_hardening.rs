//! `CYBER-DOCKER.1` (T1) — Wave-1 cyberskills: Dockerfile hardening, a
//! native Rust line-scan reimplementation of the deterministic
//! image-hardening checks harvested from
//! `vendor/anthropic-cybersecurity-skills/skills/{hardening-docker-containers-for-production,
//! performing-container-image-hardening,
//! implementing-container-image-minimal-base-with-distroless}`.
//!
//! It scans a `Dockerfile`'s text (joining `\`-continued lines into logical
//! instructions, multistage-`AS`-alias aware) and emits one `Finding` per
//! violated hardening check — the same shape a hadolint scan emits. No
//! container runtime, no image pull, no CLI subprocess.
//!
//! Scope: only the deterministic, low-false-positive SECURITY checks a
//! static text scan can enforce reliably are implemented — an unpinned /
//! `latest` base tag, running as root, a `curl|bash` remote-exec pipe, a
//! hardcoded secret in `ENV`/`ARG`, an `ADD` of a remote URL, and
//! `apt-get install` without `--no-install-recommends`. Checks that need
//! image/runtime inspection or are heuristic (minimal-base choice,
//! multistage split, setuid-bit stripping, package-manager removal,
//! HEALTHCHECK presence) are deliberately NOT emitted here — over-flagging
//! would erode a prevention gate — and are tracked as follow-ups.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

use crate::boundary::dockerfile::{
    decode_from, env_pairs, is_literal_secret_value, logical_instructions,
};

/// `CYBER-DOCKER.1` — Dockerfile hardening gate.
#[derive(Debug)]
pub struct DockerfileHardeningValidator {
    rule_id: RuleId,
    /// `RUN ... (curl|wget) ... | (sudo) sh/bash/...` remote-exec pipe.
    curl_pipe: Regex,
    /// `apt-get install` / `apt install`.
    apt_install: Regex,
    /// An ENV/ARG key whose name signals a secret.
    secret_key: Regex,
}

impl DockerfileHardeningValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberDocker.id(),
            curl_pipe: crate::boundary::regex::compile(
                "cyberskillsDockerfileRegex",
                r"(?i)\b(curl|wget)\b[^|]*\|\s*(sudo\s+)?(ba|z|a|da)?sh\b",
            )?,
            apt_install: crate::boundary::regex::compile(
                "cyberskillsDockerfileRegex",
                r"(?i)\bapt(-get)?\s+install\b",
            )?,
            secret_key: crate::boundary::regex::compile(
                "cyberskillsDockerfileRegex",
                r"(?i)(password|passwd|secret|token|api[_-]?key|access[_-]?key|private[_-]?key|credential|aws[_-]?secret)",
            )?,
        })
    }
}

impl Validator for DockerfileHardeningValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let instructions = logical_instructions(input.source.as_str());
        // A Dockerfile must have at least one FROM; otherwise this is not a
        // Dockerfile and none of the checks apply.
        if !instructions.iter().any(|i| i.keyword == "FROM") {
            return Vec::new();
        }

        let mut findings = Vec::new();
        let Some(emit) = crate::boundary::finding::ValidationFindingFactory::new(
            &self.rule_id,
            "Dockerfile violates a hardening check",
        ) else {
            return findings;
        };
        let mut stage_aliases: Vec<String> = Vec::new();
        let mut user_instructions: Vec<(&str, u32)> = Vec::new();

        for instr in &instructions {
            match instr.keyword.as_str() {
                "FROM" => {
                    let (image, alias) = decode_from(&instr.args);
                    let image_lc = image.to_ascii_lowercase();
                    let references_stage = stage_aliases.contains(&image_lc);
                    if let Some(a) = alias {
                        stage_aliases.push(a);
                    }
                    // A FROM that copies from a prior build stage, or the
                    // `scratch` pseudo-image, has no upstream tag to pin.
                    if references_stage || image_lc == "scratch" {
                        continue;
                    }
                    let pinned_by_digest = image.contains("@sha256:");
                    // Docker image tags may contain uppercase characters.
                    // `LATEST` is just as mutable as the conventional
                    // lowercase spelling, so compare the normalized image
                    // reference rather than the original source text.
                    let is_latest = image_lc.ends_with(":latest");
                    let has_tag = image
                        .rsplit('/')
                        .next()
                        .is_some_and(|last| last.contains(':'));
                    if is_latest {
                        findings.extend(emit.finding(
                            &input,
                            instr.line,
                            Severity::Error,
                            format!(
                                "base image `{image}` uses the mutable `:latest` tag. Fix: pin a \
                                 specific version (ideally `image:tag@sha256:...`)."
                            ),
                        ));
                    } else if !has_tag && !pinned_by_digest {
                        findings.extend(emit.finding(
                            &input,
                            instr.line,
                            Severity::Error,
                            format!(
                                "base image `{image}` has no version tag (implicitly `latest`). \
                                 Fix: pin a specific version (ideally `image:tag@sha256:...`)."
                            ),
                        ));
                    }
                }
                "USER" => {
                    let user = instr.args.split_whitespace().next().unwrap_or("");
                    user_instructions.push((user, instr.line));
                }
                "RUN" => {
                    if self.curl_pipe.is_match(&instr.args) {
                        findings.extend(emit.finding(
                            &input,
                            instr.line,
                            Severity::Error,
                            "RUN pipes a downloaded script straight into a shell \
                             (`curl|wget ... | sh`), executing unverified remote code at build \
                             time. Fix: download, verify a checksum/signature, then run.",
                        ));
                    }
                    if self.apt_install.is_match(&instr.args)
                        && !instr.args.contains("--no-install-recommends")
                    {
                        findings.extend(emit.finding(
                            &input,
                            instr.line,
                            Severity::Warning,
                            "`apt-get install` without `--no-install-recommends` pulls extra \
                             packages, enlarging the attack surface. Fix: add \
                             `--no-install-recommends`.",
                        ));
                    }
                }
                "ADD" => {
                    let json_source = instr
                        .args
                        .trim_start()
                        .strip_prefix("[\"")
                        .and_then(|rest| rest.split_once('"'))
                        .map(|(source, _)| source);
                    let src = json_source
                        .unwrap_or_else(|| instr.args.split_whitespace().next().unwrap_or(""));
                    if src.starts_with("http://") || src.starts_with("https://") {
                        findings.extend(emit.finding(
                            &input,
                            instr.line,
                            Severity::Error,
                            format!(
                                "ADD fetches a remote URL `{src}` (no checksum/TLS-pin \
                                 verification, and ADD auto-extracts). Fix: use `curl`/`wget` with \
                                 a verified checksum, or a pinned package."
                            ),
                        ));
                    }
                }
                "ENV" | "ARG" => {
                    for (key, value) in env_pairs(&instr.args) {
                        if !self.secret_key.is_match(&key) {
                            continue;
                        }
                        if value.as_deref().is_some_and(is_literal_secret_value) {
                            findings.extend(emit.finding(
                                &input,
                                instr.line,
                                Severity::Error,
                                format!(
                                    "`{}` sets a hardcoded secret in `{key}` — it is baked into \
                                     the image layers and leaks to anyone who pulls it. Fix: \
                                     inject secrets at runtime (mounted file / secret store), \
                                     never in ENV/ARG.",
                                    instr.keyword
                                ),
                            ));
                        }
                    }
                }
                _ => {}
            }
        }

        // Runs-as-root: no USER at all, or the last USER is root / uid 0.
        match user_instructions.last() {
            None => findings.extend(emit.finding(
                &input,
                1,
                Severity::Error,
                "no `USER` instruction — the container runs as root. Fix: add a non-root \
                 `USER` (e.g. a dedicated uid >= 10000).",
            )),
            Some((user, line)) => {
                let u = user.trim();
                let root = u == "root" || u == "0" || u.starts_with("root:") || u.starts_with("0:");
                if root {
                    findings.extend(emit.finding(
                        &input,
                        *line,
                        Severity::Error,
                        format!(
                            "final `USER {u}` runs the container as root. Fix: switch to a \
                             non-root user before the entrypoint."
                        ),
                    ));
                }
            }
        }

        findings
    }
}
