//! `CYBER-GHA.1` (T1) — harvest target: `vendor/anthropic-cybersecurity-skills/
//! skills/securing-github-actions-workflows/scripts/{agent.py,process.py}`
//! and the accompanying `SKILL.md` Step 1-4 INSECURE/SECURE examples.
//!
//! Ports four of the vendor's concrete misconfiguration checks
//! (`agent.py`'s `check_sha_pinning`/`check_permissions`/
//! `check_script_injection`/`check_dangerous_triggers`, sharpened against
//! `process.py`'s `check_action_pinning`/`check_permissions`/
//! `check_script_injection`/`check_pr_target`) into a single deterministic,
//! offline validator with no `PyYAML` dependency and no CLI subprocess:
//!
//! 1. **Broad permissions** (`process.py::check_permissions`,
//!    `agent.py::check_permissions`, SKILL.md Step 2): a top-level
//!    `permissions: write-all`, or a top-level `permissions:` block granting
//!    `contents: write` to the whole workflow — as opposed to a scoped
//!    per-job `permissions:` block, which the vendor's own SECURE example
//!    uses for `deployments: write`/`id-token: write` and is deliberately
//!    NOT flagged here (top-level-only, per the workpack spec).
//! 2. **Unpinned action** (`process.py::check_action_pinning`,
//!    `agent.py::check_sha_pinning`, SKILL.md Step 1): a `uses:` reference
//!    whose `@ref` is not a full 40-hex-char commit SHA — i.e. a mutable
//!    branch/tag such as `@main`/`@v4`. Local actions (`uses: ./...`) and
//!    Docker refs (`uses: docker://...@sha256:...`) are out of scope, per
//!    both vendor scripts' own skip conditions (`process.py`'s
//!    `uses.startswith("./")`) and the workpack's explicit carve-out.
//! 3. **Script injection** (`process.py::check_script_injection`,
//!    `agent.py::check_script_injection`, SKILL.md Step 3): a
//!    `${{ github.event.* }}` or `${{ github.head_ref }}` expression
//!    interpolated directly into a `run:` shell line or block — the same
//!    `DANGEROUS_CONTEXTS` family both vendor scripts check for
//!    (`github.event.pull_request.title`/`.body`, `.issue.title`/`.body`,
//!    `.comment.body`, `.review.body`, `github.head_ref`), generalized to
//!    the whole `github.event.*` namespace rather than an exact-string
//!    allowlist so paraphrases of the same attacker-controlled fields are
//!    still caught. `${{ env.* }}`/`${{ secrets.* }}`/`${{ matrix.* }}` are
//!    not `github.event.*` and are not flagged, matching the vendor's own
//!    SECURE fix of routing untrusted input through `env:` first.
//! 4. **`pull_request_target` + untrusted checkout** (`process.py::
//!    check_pr_target`, `agent.py::check_dangerous_triggers`, SKILL.md
//!    Step 4): a whole-source correlation check, not a single-line
//!    predicate — `on: pull_request_target` combined ANYWHERE in the file
//!    with an `actions/checkout` step whose `with: ref:` is
//!    `github.event.pull_request.head.sha`/`.head.ref` runs untrusted fork
//!    code with the base repo's privileged `GITHUB_TOKEN`.

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// `CYBER-GHA.1` — insecure GitHub Actions workflow configuration.
pub struct GithubActionsSecurityValidator {
    rule_id: RuleId,
    /// A `contents: write` key nested under a top-level `permissions:`
    /// block header (check 1, block form).
    top_level_contents_write: Regex,
    /// `uses: <owner>/<repo>@<ref>`, optionally quoted (check 2).
    uses_ref: Regex,
    /// The PINNED (clean) form: exactly 40 hex chars, the whole ref.
    pinned_sha: Regex,
    /// `${{ ... github.event.<path> ... }}` / `${{ ... github.head_ref ... }}`
    /// (check 3).
    injection_expr: Regex,
    /// `ref:` set to the PR head commit/branch, typically under a
    /// checkout step's `with:` (check 4, untrusted-checkout half).
    checkout_pr_head_ref: Regex,
}

impl GithubActionsSecurityValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "CYBER-GHA.1".parse()?,
            top_level_contents_write: Regex::new(r"(?i)^\s*contents:\s*write\b")
                .map_err(|err| DecodeError::new("cyberskillsGhaContentsWrite", err.to_string()))?,
            // Ref runs to the next whitespace/quote/comment so a trailing
            // `# v4.1.1` comment is never folded into the captured ref.
            uses_ref: Regex::new(r##"(?i)uses:\s*['"]?([^\s'"@]+)@([^\s'"#]+)"##)
                .map_err(|err| DecodeError::new("cyberskillsGhaUsesRef", err.to_string()))?,
            pinned_sha: Regex::new(r"^[0-9a-fA-F]{40}$")
                .map_err(|err| DecodeError::new("cyberskillsGhaPinnedSha", err.to_string()))?,
            // Allows a wrapping function call (e.g. `format(...)`) between
            // the `${{`/`}}` delimiters and the dangerous context.
            injection_expr: Regex::new(
                r"\$\{\{[^}]*\bgithub\.(?:event\.[A-Za-z0-9_.]+|head_ref)\b[^}]*\}\}",
            )
            .map_err(|err| DecodeError::new("cyberskillsGhaInjectionExpr", err.to_string()))?,
            checkout_pr_head_ref: Regex::new(
                r#"(?i)ref:\s*['"]?\$\{\{\s*github\.event\.pull_request\.head\.(?:sha|ref)\s*\}\}"#,
            )
            .map_err(|err| DecodeError::new("cyberskillsGhaCheckoutHeadRef", err.to_string()))?,
        })
    }
}

impl Validator for GithubActionsSecurityValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        let mut in_top_level_permissions = false;
        let mut in_run_block = false;

        for (index, line) in input.source.lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
            let trimmed = line.trim_start();
            let leading_ws = line.len() - trimmed.len();
            let is_top_level_key = leading_ws == 0 && !trimmed.trim_end().is_empty();

            // --- Check 1: broad permissions ---
            if is_top_level_key {
                if let Some(rest) = trimmed.strip_prefix("permissions:") {
                    let value = rest.split('#').next().unwrap_or_default().trim();
                    if value.eq_ignore_ascii_case("write-all") {
                        // CLONE-JUSTIFICATION: each emitted diagnostic owns rule and file identity after validation returns.
                        findings.push(Finding {
                            rule_id: self.rule_id.clone(),
                            severity: Severity::Error,
                            title: "GitHub Actions workflow grants write-all permissions"
                                .to_owned(),
                            detail: "Top-level `permissions: write-all` grants the GITHUB_TOKEN \
                                      write access to every resource the workflow can touch. \
                                      Fix: replace with `permissions: {}` at the workflow level \
                                      and grant only the specific scopes each job needs (e.g. \
                                      `contents: read`)."
                                .to_owned(),
                            // CLONE-JUSTIFICATION: diagnostic owns the borrowed file path.
                            file: input.file.clone(),
                            line: line_number,
                            snippet: Some(line.to_owned()),
                        });
                    }
                    in_top_level_permissions = value.is_empty();
                } else {
                    in_top_level_permissions = false;
                }
            } else if in_top_level_permissions && self.top_level_contents_write.is_match(line) {
                // CLONE-JUSTIFICATION: each emitted diagnostic owns rule and file identity after validation returns.
                findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "GitHub Actions workflow grants broad top-level write permission"
                        .to_owned(),
                    detail: "The top-level `permissions:` block grants `contents: write` to \
                              every job in the workflow. Fix: remove `contents: write` from the \
                              workflow-level `permissions:` block and grant it only on the \
                              specific job(s) that need it."
                        .to_owned(),
                    // CLONE-JUSTIFICATION: diagnostic owns the borrowed file path.
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(line.to_owned()),
                });
            }

            // --- Check 2: unpinned action `uses:` ---
            let is_docker_ref = line.contains("docker://");
            if let Some(captures) = self.uses_ref.captures(line) {
                let action = captures.get(1).map(|m| m.as_str()).unwrap_or_default();
                let reference = captures.get(2).map(|m| m.as_str()).unwrap_or_default();
                let is_local_action = action.starts_with("./");
                let is_pinned = self.pinned_sha.is_match(reference);
                if !is_docker_ref && !is_local_action && !is_pinned {
                    // CLONE-JUSTIFICATION: each emitted diagnostic owns rule and file identity after validation returns.
                    findings.push(Finding {
                        rule_id: self.rule_id.clone(),
                        severity: Severity::Warning,
                        title: "GitHub Action referenced by a mutable ref, not a pinned SHA"
                            .to_owned(),
                        detail: format!(
                            "`{action}@{reference}` is not pinned to a full 40-character commit \
                             SHA. A mutable branch or tag can be overwritten by an attacker who \
                             compromises the action's repository, silently pulling malicious \
                             code into this workflow. Fix: pin to the immutable commit SHA, \
                             e.g. `{action}@<40-char-sha>  # {reference}`."
                        ),
                        // CLONE-JUSTIFICATION: diagnostic owns the borrowed file path.
                        file: input.file.clone(),
                        line: line_number,
                        snippet: Some(line.to_owned()),
                    });
                }
            }

            // --- Check 3: script injection on a `run:` line or block ---
            let is_run_key = trimmed.starts_with("run:");
            let currently_in_run = is_run_key || in_run_block;
            if currently_in_run && self.injection_expr.is_match(line) {
                // CLONE-JUSTIFICATION: each emitted diagnostic owns rule and file identity after validation returns.
                findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "Untrusted GitHub event data interpolated into a shell command"
                        .to_owned(),
                    detail: "A `${{ github.event.* }}`/`${{ github.head_ref }}` expression is \
                              interpolated directly into a `run:` step. These values are \
                              attacker-controlled (PR title/body, issue/comment text, branch \
                              name) and are substituted into the shell command BEFORE it runs, \
                              allowing arbitrary command injection. Fix: pass the value through \
                              an `env:` variable and reference it as a shell variable (e.g. \
                              `${PR_TITLE}`) instead of interpolating the expression directly."
                        .to_owned(),
                    // CLONE-JUSTIFICATION: diagnostic owns the borrowed file path.
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(line.to_owned()),
                });
            }
            if is_run_key {
                in_run_block = true;
            } else if in_run_block {
                let is_list_item = trimmed.starts_with('-');
                let is_new_key = trimmed.contains(':') && !is_list_item;
                if is_list_item || is_new_key {
                    in_run_block = false;
                }
            }
        }

        // --- Check 4: pull_request_target + untrusted checkout (whole-source correlation) ---
        let has_pr_target_trigger = input.source.contains("pull_request_target");
        let has_checkout_action = input.source.contains("actions/checkout");
        let mut head_ref_hit: Option<(u32, &str)> = None;
        if has_pr_target_trigger && has_checkout_action {
            for (index, line) in input.source.lines().enumerate() {
                if self.checkout_pr_head_ref.is_match(line) {
                    let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
                    head_ref_hit = Some((line_number, line));
                    break;
                }
            }
        }
        if let Some((line_number, matched_line)) = head_ref_hit {
            // CLONE-JUSTIFICATION: delayed workflow finding owns rule and file identity beyond source scanning.
            findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "pull_request_target checks out untrusted PR head code".to_owned(),
                detail: "This workflow triggers on `pull_request_target` (which runs with the \
                          base repository's privileged GITHUB_TOKEN and secrets) AND checks out \
                          the PR's own head commit/branch via `actions/checkout` with `ref: \
                          ${{ github.event.pull_request.head.sha }}`/`.head.ref`. Any later \
                          step now executes untrusted fork code with privileged credentials. \
                          Fix: use the `pull_request` trigger instead, or if \
                          `pull_request_target` is required, never check out the PR head — \
                          only check out the base branch, and gate privileged steps behind a \
                          maintainer label/review."
                    .to_owned(),
                // CLONE-JUSTIFICATION: diagnostic owns the borrowed file path.
                file: input.file.clone(),
                line: line_number,
                snippet: Some(matched_line.to_owned()),
            });
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::GithubActionsSecurityValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_github_actions() -> Result<(), Box<dyn std::error::Error>> {
        let validator = GithubActionsSecurityValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/ci.github-actions/bad/workflow.yml",
            "tests/fixtures/cyberskills/ci.github-actions/good/workflow.yml",
        )?;
        Ok(())
    }
}
