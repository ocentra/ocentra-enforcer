//! `CYBER-SQLI-SOURCE.1` (T1) — source-side SQL-injection-by-string-
//! construction detector.
//!
//! Harvest note: the harvest target,
//! `vendor/anthropic-cybersecurity-skills/skills/exploiting-sql-injection-vulnerabilities/`,
//! is a black-box penetration-testing playbook: its `SKILL.md` documents
//! HTTP payload techniques (error/boolean/time-based blind injection,
//! `sqlmap` usage) and its `scripts/agent.py` sends live HTTP requests and
//! greps *responses* for database error signatures (`SQL_ERRORS`,
//! L21-27) — there is no source-code query-construction regex table to
//! port (the same shape of gap `cmd_injection.rs` documents for its own
//! harvest target). What the skill DOES specify, concretely and
//! unambiguously, is the vulnerable-vs-fixed pair a source-level detector
//! should tell apart:
//! - The "Key Concepts" table defines a **Parameterized Query** as "a
//!   prepared SQL statement where user input is passed as parameters
//!   rather than concatenated into the query string, preventing
//!   injection" (`SKILL.md` L137).
//! - The healthcare-portal scenario's root cause is that the app
//!   "concatenates the ... URL parameter directly into a SQL query
//!   without parameterization" (`SKILL.md` L182-186).
//! - The Output Format's Remediation block gives the canonical
//!   vulnerable/fixed pair verbatim (`SKILL.md` L197-201):
//!   `VULNERABLE:  $query = "SELECT * FROM appointments WHERE id = " . $_GET['id'];`
//!   vs.
//!   `SECURE:      $stmt = $pdo->prepare("SELECT * FROM appointments WHERE id = ?"); $stmt->execute([$_GET['id']]);`
//!
//! This validator implements that vulnerable/fixed distinction directly
//! against source lines (no HTTP, no CLI subprocess): it looks for a DB
//! sink call —
//! - Python: `.execute(`, `.executemany(`, `.raw(` (incl. `cursor.execute(`
//!   and Django's `Model.objects.raw(`)
//! - JS/Node: `.query(`, `.raw(` (incl. `sequelize.query(`, `knex.raw(`)
//!
//! — whose argument splices a variable into SQL text via one of:
//! 1. an f-string containing `{` and a SQL keyword,
//! 2. `+` string concatenation with a SQL keyword present,
//! 3. the `%` string-formatting operator applied directly after a SQL
//!    string literal (the unsafe `"... %s" % var` form, as opposed to the
//!    safe `execute(sql, params)` two-argument form),
//! 4. `.format(` chained directly onto a SQL string literal,
//! 5. a JS/Node template literal containing `${` and a SQL keyword.
//!
//! A SQL keyword is one of `SELECT`/`INSERT`/`UPDATE`/`DELETE`/`FROM`/
//! `WHERE`/`UNION`/`DROP` (case-insensitive, word-bounded so e.g.
//! `UPDATE_COUNTER` does not match `UPDATE`). Parameterized calls
//! (`?`, `%s` + a separate params tuple, `$1`, `:name`, `@name`) and ORM
//! builder calls that never build a raw SQL string are, by construction,
//! not matched by any of the five shapes above.

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// `CYBER-SQLI-SOURCE.1` — flags a DB sink call whose SQL argument is
/// assembled via f-string/template-literal interpolation, `+`
/// concatenation, `%` string formatting, or `.format()`, rather than
/// passed as a bound parameter.
pub struct SqlInjectionSourceValidator {
    rule_id: RuleId,
    sink: Regex,
    keyword: Regex,
    fstring_prefix: Regex,
    percent_after_quote: Regex,
    dot_format_after_quote: Regex,
}

impl SqlInjectionSourceValidator {
    pub fn new() -> Result<Self, DecodeError> {
        // Sink call sites: Python `.execute(`/`.executemany(`/`.raw(` and
        // JS/Node `.query(`/`.raw(`, whichever attribute the callee has
        // (`cursor.execute(`, `Model.objects.raw(`, `sequelize.query(`,
        // `knex.raw(`, ...). Capture group 1 is the (lazily matched, single
        // paren depth) argument text.
        let sink = Regex::new(r"\.(?:execute|executemany|raw|query)\s*\((.*?)\)")
            .map_err(|err| DecodeError::new("cyberskillsSqliSourceSink", err.to_string()))?;
        // Word-bounded so `UPDATE_COUNTER` or `FROMAGE` do not match.
        let keyword = Regex::new(r"(?i)\b(?:SELECT|INSERT|UPDATE|DELETE|FROM|WHERE|UNION|DROP)\b")
            .map_err(|err| DecodeError::new("cyberskillsSqliSourceKeyword", err.to_string()))?;
        let fstring_prefix = Regex::new(r#"^[fF]["']"#).map_err(|err| {
            DecodeError::new("cyberskillsSqliSourceFstringPrefix", err.to_string())
        })?;
        // A quote immediately (modulo whitespace) followed by the `%`
        // operator: the unsafe `"... %s" % var` shape. A `%s` placeholder
        // living INSIDE the string (before its closing quote) never
        // matches this, and neither does the safe two-argument
        // `execute(sql, (params,))` call (a comma, not `%`, follows the
        // closing quote there).
        let percent_after_quote = Regex::new(r#"["']\s*%\s*"#).map_err(|err| {
            DecodeError::new("cyberskillsSqliSourcePercentAfterQuote", err.to_string())
        })?;
        // A quote immediately (modulo whitespace) followed by `.format(`:
        // `"...".format(var)` chained straight onto the SQL literal.
        let dot_format_after_quote = Regex::new(r#"["']\s*\.\s*format\s*\("#).map_err(|err| {
            DecodeError::new("cyberskillsSqliSourceDotFormatAfterQuote", err.to_string())
        })?;
        Ok(Self {
            rule_id: "CYBER-SQLI-SOURCE.1".parse()?,
            sink,
            keyword,
            fstring_prefix,
            percent_after_quote,
            dot_format_after_quote,
        })
    }
}

impl Validator for SqlInjectionSourceValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (index, line) in input.source.lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
            let Some(captures) = self.sink.captures(line) else {
                continue;
            };
            let argument = captures.get(1).map(|m| m.as_str()).unwrap_or("").trim();
            if argument.is_empty() {
                continue;
            }

            let mut matched_labels: Vec<&str> = Vec::new();
            if self.fstring_prefix.is_match(argument)
                && argument.contains('{')
                && self.keyword.is_match(argument)
            {
                matched_labels.push("Python f-string SQL interpolation");
            }
            let is_template_literal = argument.starts_with('`')
                && argument.contains("${")
                && self.keyword.is_match(argument);
            if is_template_literal {
                matched_labels.push("JS/Node template-literal SQL interpolation");
            }
            if self.percent_after_quote.is_match(argument) && self.keyword.is_match(argument) {
                matched_labels.push("Python `%` string-formatting operator applied to SQL");
            }
            if self.dot_format_after_quote.is_match(argument) && self.keyword.is_match(argument) {
                matched_labels.push(".format() chained onto a SQL string literal");
            }
            if argument.contains('+') && self.keyword.is_match(argument) {
                matched_labels.push("string concatenation into SQL");
            }

            if matched_labels.is_empty() {
                continue;
            }

            findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "SQL query built by unsafe string construction".to_owned(),
                detail: format!(
                    "A DB sink call assembles its SQL argument via: {}. An attacker-controlled \
                     value spliced directly into SQL text can change the statement's meaning. \
                     Fix: use a parameterized query / prepared statement (placeholders such as \
                     `?`, `%s` with a separate params tuple, `$1`, `:name`, or `@name`) instead \
                     of splicing a variable into the SQL string.",
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

    use super::SqlInjectionSourceValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_sqli_source() -> Result<(), Box<dyn std::error::Error>> {
        let validator = SqlInjectionSourceValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/cyberskills/web.sql-injection-source/bad/vuln.py",
            "tests/fixtures/cyberskills/web.sql-injection-source/good/safe.py",
        )?;
        Ok(())
    }
}
