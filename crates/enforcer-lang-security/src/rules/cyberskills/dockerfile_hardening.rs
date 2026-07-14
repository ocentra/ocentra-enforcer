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

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// One logical Dockerfile instruction (after joining `\` continuations).
struct Instruction {
    keyword: String,
    args: String,
    line: u32,
}

/// Join backslash-continued lines into logical instructions, dropping blank
/// and `#`-comment lines. Keeps the 1-based line number of each
/// instruction's first line.
fn logical_instructions(source: &str) -> Vec<Instruction> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut start_line = 0u32;
    for (index, raw) in source.lines().enumerate() {
        let line_no = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
        let line = raw.trim_end();
        let trimmed = line.trim_start();
        if buf.is_empty() {
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            start_line = line_no;
        }
        if let Some(without_slash) = line.strip_suffix('\\') {
            buf.push_str(without_slash);
            buf.push(' ');
            continue;
        }
        buf.push_str(line);
        let logical = buf.trim().to_owned();
        buf.clear();
        if logical.is_empty() {
            continue;
        }
        let (keyword, args) = match logical.split_once(char::is_whitespace) {
            Some((kw, rest)) => (kw.to_ascii_uppercase(), rest.trim().to_owned()),
            None => (logical.to_ascii_uppercase(), String::new()),
        };
        out.push(Instruction {
            keyword,
            args,
            line: start_line,
        });
    }
    if !buf.trim().is_empty() {
        let logical = buf.trim().to_owned();
        let (keyword, args) = match logical.split_once(char::is_whitespace) {
            Some((kw, rest)) => (kw.to_ascii_uppercase(), rest.trim().to_owned()),
            None => (logical.to_ascii_uppercase(), String::new()),
        };
        out.push(Instruction {
            keyword,
            args,
            line: start_line,
        });
    }
    out
}

fn compile(pattern: &str) -> Result<Regex, DecodeError> {
    Regex::new(pattern)
        .map_err(|err| DecodeError::new("cyberskillsDockerfileRegex", err.to_string()))
}

/// `CYBER-DOCKER.1` — Dockerfile hardening gate.
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
            rule_id: "CYBER-DOCKER.1".parse()?,
            curl_pipe: compile(r"(?i)\b(curl|wget)\b[^|]*\|\s*(sudo\s+)?(ba|z|a|da)?sh\b")?,
            apt_install: compile(r"(?i)\bapt(-get)?\s+install\b")?,
            secret_key: compile(
                r"(?i)(password|passwd|secret|token|api[_-]?key|access[_-]?key|private[_-]?key|credential|aws[_-]?secret)",
            )?,
        })
    }

    fn finding(
        &self,
        input: &ValidationInput<'_>,
        line: u32,
        sev: Severity,
        detail: String,
    ) -> Finding {
        Finding {
            rule_id: self.rule_id.clone(),
            severity: sev,
            title: "Dockerfile violates a hardening check".to_owned(),
            detail,
            file: input.file.clone(),
            line,
            snippet: None,
        }
    }
}

/// Parse a `FROM` argument into (image_ref, optional stage alias).
fn parse_from(args: &str) -> (&str, Option<String>) {
    let mut parts = args.split_whitespace();
    let mut image = parts.next().unwrap_or("");
    // Docker permits an optional platform selector before the image, e.g.
    // `FROM --platform=$BUILDPLATFORM rust:1.88 AS builder`. The selector is
    // a build-stage option, not an image reference, so it must not be checked
    // for tag pinning or stage-alias matching.
    while image.starts_with("--") {
        if image == "--platform" {
            let _platform = parts.next();
        }
        image = parts.next().unwrap_or("");
    }
    // ` AS <alias>` (case-insensitive) marks a build-stage name.
    let mut alias = None;
    let tokens: Vec<&str> = args.split_whitespace().collect();
    if let Some(pos) = tokens.iter().position(|t| t.eq_ignore_ascii_case("as")) {
        if let Some(name) = tokens.get(pos + 1) {
            alias = Some(name.to_ascii_lowercase());
        }
    }
    (image, alias)
}

/// Split an `ENV`/`ARG` argument string into (key, optional literal value)
/// pairs, handling both `KEY=value ...` and the legacy `KEY value` form.
fn env_pairs(args: &str) -> Vec<(String, Option<String>)> {
    let trimmed = args.trim();
    if trimmed.contains('=') {
        trimmed
            .split_whitespace()
            .filter_map(|tok| {
                tok.split_once('=')
                    .map(|(k, v)| (k.to_owned(), Some(v.trim_matches(['"', '\'']).to_owned())))
            })
            .collect()
    } else {
        // Legacy `ENV KEY the rest is the value`; `ARG KEY` (no value).
        match trimmed.split_once(char::is_whitespace) {
            Some((k, v)) => vec![(
                k.to_owned(),
                Some(v.trim().trim_matches(['"', '\'']).to_owned()),
            )],
            None => vec![(trimmed.to_owned(), None)],
        }
    }
}

/// A value is a hardcoded literal secret if it is non-empty, is not a
/// build-arg/variable reference (`$FOO`), and is not an obvious placeholder.
fn is_literal_secret_value(value: &str) -> bool {
    let v = value.trim();
    if v.is_empty() || v.starts_with('$') {
        return false;
    }
    let placeholder = v.eq_ignore_ascii_case("changeme")
        || (v.starts_with('<') && v.ends_with('>'))
        || (v.starts_with('{') && v.ends_with('}'));
    !placeholder
}

impl Validator for DockerfileHardeningValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let instructions = logical_instructions(input.source);
        // A Dockerfile must have at least one FROM; otherwise this is not a
        // Dockerfile and none of the checks apply.
        if !instructions.iter().any(|i| i.keyword == "FROM") {
            return Vec::new();
        }

        let mut findings = Vec::new();
        let mut stage_aliases: Vec<String> = Vec::new();
        let mut user_instructions: Vec<(&str, u32)> = Vec::new();

        for instr in &instructions {
            match instr.keyword.as_str() {
                "FROM" => {
                    let (image, alias) = parse_from(&instr.args);
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
                    let is_latest = image.ends_with(":latest");
                    let has_tag = image
                        .rsplit('/')
                        .next()
                        .is_some_and(|last| last.contains(':'));
                    if is_latest {
                        findings.push(self.finding(
                            &input,
                            instr.line,
                            Severity::Error,
                            format!(
                                "base image `{image}` uses the mutable `:latest` tag. Fix: pin a \
                                 specific version (ideally `image:tag@sha256:...`)."
                            ),
                        ));
                    } else if !has_tag && !pinned_by_digest {
                        findings.push(self.finding(
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
                        findings.push(
                            self.finding(
                                &input,
                                instr.line,
                                Severity::Error,
                                "RUN pipes a downloaded script straight into a shell \
                             (`curl|wget ... | sh`), executing unverified remote code at build \
                             time. Fix: download, verify a checksum/signature, then run."
                                    .to_owned(),
                            ),
                        );
                    }
                    if self.apt_install.is_match(&instr.args)
                        && !instr.args.contains("--no-install-recommends")
                    {
                        findings.push(
                            self.finding(
                                &input,
                                instr.line,
                                Severity::Warning,
                                "`apt-get install` without `--no-install-recommends` pulls extra \
                             packages, enlarging the attack surface. Fix: add \
                             `--no-install-recommends`."
                                    .to_owned(),
                            ),
                        );
                    }
                }
                "ADD" => {
                    let src = instr.args.split_whitespace().next().unwrap_or("");
                    if src.starts_with("http://") || src.starts_with("https://") {
                        findings.push(self.finding(
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
                            findings.push(self.finding(
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
            None => findings.push(
                self.finding(
                    &input,
                    1,
                    Severity::Error,
                    "no `USER` instruction — the container runs as root. Fix: add a non-root \
                 `USER` (e.g. a dedicated uid >= 10000)."
                        .to_owned(),
                ),
            ),
            Some((user, line)) => {
                let u = user.trim();
                let root = u == "root" || u == "0" || u.starts_with("root:") || u.starts_with("0:");
                if root {
                    findings.push(self.finding(
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
