//! `CYBER-SSTI.1` (T1) — server-side template injection (SSTI) sink
//! detector.
//!
//! Harvest target: `vendor/anthropic-cybersecurity-skills/skills/
//! exploiting-template-injection-vulnerabilities/SKILL.md`. That skill is a
//! penetration-testing playbook for probing a *running* application with
//! `{{7*7}}`-style detection payloads and engine-specific RCE gadgets
//! (Jinja2 `config`/`cycler`/`lipsum` subclass traversal, Twig
//! `filter('system')`, Freemarker `Execute`, ...) — it has no
//! static-source detection predicate to port directly. Its "Common
//! Scenarios" section names the concrete root cause this validator ports
//! to source-code analysis (Scenario 1: "A Flask application lets users
//! customize email notification templates. The custom template is
//! rendered with Jinja2 without sandboxing..."): a template engine is fed
//! *attacker-influenceable template text*, as opposed to a static/named
//! template with untrusted data passed only as separate render
//! arguments, which the engine's own escaping handles safely.
//!
//! Sinks covered (source scanned line by line, per-line regex-table
//! approach shared with `weak_crypto`/`cmd_injection`):
//! - Python `render_template_string(...)` (Flask/Jinja2) — flagged only
//!   when its template-text argument is dynamically built: an f-string, a
//!   `+` concatenation, a `.format(...)` call, or `%`-formatting.
//! - Python/Mako `Template(...).render(...)` (jinja2/mako/string.Template,
//!   including a `MakoTemplate` import alias) — flagged only when the
//!   `Template(...)` argument is dynamically built (f-string /
//!   concatenation / bare variable) and `.render(` is called on the same
//!   line.
//! - JS `new Function(...)` — flagged only when its argument is
//!   dynamically built (backtick template-literal interpolation,
//!   concatenation, or a bare variable): the classic `Function`
//!   constructor code-gen sink.
//! - JS `<engine>.compile(...)` (Handlebars-style template compilation) —
//!   flagged only when its argument is a backtick template literal that
//!   itself contains `${...}` interpolation, i.e. the compiled template
//!   source is built from a variable rather than passed as a static
//!   string.
//!
//! Do NOT flag: `render_template("index.html", name=user)` (a named
//! template FILE with data passed as render kwargs — the engine's own
//! escaping handles `name`), a fully static `render_template_string(...)`
//! or `Template(...).render(...)` literal, or an ordinary f-string that is
//! never passed to a template-rendering sink at all.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// Whether a sink's captured argument (capture group 1) is checked with the
/// general dynamic-template-text predicate, or with the narrower
/// template-literal-interpolation-only predicate used for `.compile(...)`.
enum SinkKind {
    /// Flag when [`is_dynamic_template`] returns `true` for the captured
    /// argument (f-string, concatenation, `.format(...)`, `%`-formatting,
    /// backtick interpolation, or a bare variable).
    DynamicTemplateText,
    /// Flag only when the captured argument is a backtick template literal
    /// that itself contains `${...}` interpolation (narrower than
    /// [`SinkKind::DynamicTemplateText`], per the `.compile(...)` predicate).
    InterpolatedLiteralOnly,
}

struct SinkPattern {
    label: &'static str,
    regex: Regex,
    kind: SinkKind,
}

/// A captured template-text argument is treated as dynamically built when it
/// contains string concatenation (`+`), a `.format(...)` call, backtick
/// interpolation (`` ` `` together with `${`), an f-string prefix
/// (`f"`/`F'`/...), `%`-formatting immediately after a string literal (e.g.
/// `"..." % title` or `"..." % (a, b)`), or is a bare variable/expression
/// with no string-literal quote character at all. A fully static, fully
/// quoted literal (e.g. `"<h1>Hello</h1>"`) is never flagged.
fn is_dynamic_template(argument: &str, fstring_prefix: &Regex, percent_format: &Regex) -> bool {
    let trimmed = argument.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains('+') || trimmed.contains(".format(") {
        return true;
    }
    if trimmed.contains('`') && trimmed.contains("${") {
        return true;
    }
    if fstring_prefix.is_match(trimmed) || percent_format.is_match(trimmed) {
        return true;
    }
    !trimmed.contains('"') && !trimmed.contains('\'') && !trimmed.contains('`')
}

/// A captured `.compile(...)` argument is flagged only when it is itself a
/// backtick template literal containing `${...}` interpolation — a bare
/// variable or a fully static (non-backtick) string is not flagged here,
/// since `.compile(...)` is routinely called with a static or pre-loaded
/// template string and only the interpolated-literal case names the
/// template SOURCE being built from a variable.
fn is_interpolated_template_literal(argument: &str) -> bool {
    let trimmed = argument.trim();
    trimmed.contains('`') && trimmed.contains("${")
}

/// `CYBER-SSTI.1` — flags server-side template injection sinks whose
/// template-text argument is dynamically constructed from untrusted input.
pub struct TemplateInjectionValidator {
    rule_id: RuleId,
    sinks: Vec<SinkPattern>,
    fstring_prefix: Regex,
    percent_format: Regex,
}

impl TemplateInjectionValidator {
    pub fn new() -> Result<Self, DecodeError> {
        fn compile(slug: &'static str, pattern: &str) -> Result<Regex, DecodeError> {
            Regex::new(pattern).map_err(|err| DecodeError::new(slug, err.to_string()))
        }

        let sinks = vec![
            SinkPattern {
                label: "Flask/Jinja2 render_template_string(...) with a dynamically built template",
                regex: compile(
                    "cyberskillsSstiRenderTemplateString",
                    r"render_template_string\s*\((.*?)\)",
                )?,
                kind: SinkKind::DynamicTemplateText,
            },
            SinkPattern {
                label: "Template(...).render(...) with a dynamically built template (jinja2/mako/string.Template)",
                regex: compile(
                    "cyberskillsSstiTemplateRender",
                    r"Template\s*\((.*?)\)\s*\.render\s*\(",
                )?,
                kind: SinkKind::DynamicTemplateText,
            },
            SinkPattern {
                label: "JS new Function(...) built from interpolated/concatenated code text",
                regex: compile("cyberskillsSstiNewFunction", r"new\s+Function\s*\((.*?)\)")?,
                kind: SinkKind::DynamicTemplateText,
            },
            SinkPattern {
                label: "JS template-engine .compile(...) fed a backtick literal with ${...} interpolation",
                regex: compile("cyberskillsSstiCompile", r"\.compile\s*\((.*?)\)")?,
                kind: SinkKind::InterpolatedLiteralOnly,
            },
        ];

        Ok(Self {
            rule_id: "CYBER-SSTI.1".parse()?,
            sinks,
            fstring_prefix: compile("cyberskillsSstiFstringPrefix", r#"^[fF]['"]"#)?,
            percent_format: compile("cyberskillsSstiPercentFormat", r#"["']\s*%\s*[\w(]"#)?,
        })
    }
}

impl Validator for TemplateInjectionValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (index, line) in input.source.lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
            let mut matched_labels: Vec<&str> = Vec::new();

            for sink in &self.sinks {
                let Some(captures) = sink.regex.captures(line) else {
                    continue;
                };
                let argument = captures.get(1).map_or("", |m| m.as_str());
                let dynamic = match sink.kind {
                    SinkKind::DynamicTemplateText => {
                        is_dynamic_template(argument, &self.fstring_prefix, &self.percent_format)
                    }
                    SinkKind::InterpolatedLiteralOnly => is_interpolated_template_literal(argument),
                };
                if dynamic && !matched_labels.contains(&sink.label) {
                    matched_labels.push(sink.label);
                }
            }

            if matched_labels.is_empty() {
                continue;
            }

            findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "Template rendered from a dynamically constructed source (SSTI)".to_owned(),
                detail: format!(
                    "Sink(s) matched: {}. The template TEXT itself is built from a variable \
                     (f-string, concatenation, `.format()`/`%`-formatting, or backtick \
                     interpolation), so an attacker who controls that variable can inject \
                     template directives and achieve server-side template injection (SSTI), up \
                     to remote code execution. Fix: render a named/static template and pass \
                     untrusted data only as separate render arguments (e.g. \
                     `render_template(\"file.html\", name=user)` or \
                     `Template(\"static text\").render(name=user)`), never build the template \
                     source string from untrusted input.",
                    matched_labels.join(", ")
                ),
                file: input.file.clone(),
                line: line_number,
                snippet: Some(line.to_owned()),
            });
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::TemplateInjectionValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_ssti() -> Result<(), Box<dyn std::error::Error>> {
        let validator = TemplateInjectionValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/web.template-injection/bad/vuln.py",
            "tests/fixtures/cyberskills/web.template-injection/good/safe.py",
        )?;
        Ok(())
    }
}
