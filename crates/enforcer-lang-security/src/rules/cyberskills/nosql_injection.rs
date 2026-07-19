//! `CYBER-NOSQL-INJECT.1` (T1) — harvest target: the vendored
//! `exploiting-nosql-injection-vulnerabilities` cyberskill
//! (`vendor/anthropic-cybersecurity-skills/skills/exploiting-nosql-injection-vulnerabilities/SKILL.md`
//! and its `scripts/agent.py` / `scripts/process.py`). The vendor skill is a
//! live-target attack playbook: its scripts fire live HTTP requests
//! carrying MongoDB operator payloads (`{"$ne": ""}`, `{"$gt": ""}`,
//! `{"$regex": ".*"}`, `{"$where": "1==1"}`, from `agent.py`'s
//! `NOSQL_PAYLOADS_JSON` and `process.py`'s `test_operator_injection`/
//! `test_where_injection`) at a running application to demonstrate
//! authentication bypass, blind data extraction, and JavaScript injection —
//! there is no inline source-scanning predicate to port verbatim. This
//! validator instead implements the deterministic, Semgrep-style SOURCE
//! check for the vulnerability classes the skill's "Key Concepts" table
//! names (`$where` server-side JS injection, "Operator Injection", "Type
//! Juggling" object-vs-string confusion, and the `$regex` blind-extraction
//! operator named throughout Steps 3-4), narrowed to concrete call-site
//! shapes so only genuine unsanitized request-data flow triggers, per the
//! PREVENTION-gate false-positive budget:
//!
//! - `$where` given a string built from a variable — a JS template literal
//!   with `${...}` interpolation, a JS/Python string literal immediately
//!   concatenated (`+`) with a variable, or a Python f-string (`f"..."`) —
//!   since MongoDB executes `$where` as server-side JavaScript (SKILL.md
//!   Step 4 "Exploit JavaScript Injection via `$where`";
//!   `process.py::test_where_injection`). A `$where` that is a fully static
//!   string literal is left clean.
//! - A raw `req.body`/`req.query`/`req.params` (Express) or
//!   `request.json`/`request.get_json()`/`request.args`/`request.form`
//!   (Flask/pymongo) object passed straight into a query method
//!   (`.find`/`.findOne`/...) — the classic operator-injection shape named
//!   in SKILL.md's "Operator Injection"/"Type Juggling" concepts and
//!   `agent.py`'s `NOSQL_PAYLOADS_JSON` (`{"$ne": ...}`, `{"$gt": ...}`,
//!   `{"$exists": ...}`), since an attacker who controls the WHOLE request
//!   object can substitute an operator object for the expected string.
//! - An individual `req.body.*`/`req.query.*`/`req.params.*` property used
//!   directly as a query-filter value in the same call (e.g.
//!   `.find({ username: req.body.username })`) WITHOUT an explicit type
//!   cast (`String(...)`, `Number(...)`, `parseInt(...)`, `parseFloat(...)`,
//!   or any other wrapping validator call) — the same operator-injection
//!   risk at property granularity; a cast forces the value to a primitive,
//!   closing off the `{"$gt": ""}`-shaped payload.
//! - `$regex` whose value is a bare, un-cast request property — SKILL.md
//!   Steps 3-4's boolean/blind-extraction technique
//!   (`{"password": {"$regex": "^a"}}`), ported from
//!   `process.py::blind_extract_field` / `enumerate_usernames`.
//! - `JSON.parse(req.body...)` / `JSON.parse(req.query...)` used to
//!   deserialize raw request text straight into a query object — bypasses
//!   the framework's normal body-parser typing and hands the attacker the
//!   same "pass an object where a string is expected" primitive.
//! - pymongo equivalents: `collection.find({"$where": f"..."})` and
//!   `collection.find(request.json)` / `find(request.get_json())`.
//!
//! Every sink here is a high-confidence, attacker-reachable query-shaping
//! primitive, so all findings are `Severity::Error` (matches the PREDICATE
//! given for this rule). Detection deliberately does not gate on file
//! extension: the discriminator is the unsanitized request-derived value
//! flowing into a query operator/`$where`/`$regex`, which is the same
//! source-level shape whether the file is `.js`/`.ts`/`.py`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

use crate::boundary::pattern::{RemediationPattern, RemediationPatternSource as NoSqlSink};

/// Sinks that are unsafe the moment the regex matches — the match itself
/// proves the request-derived/interpolated shape, so no further per-line
/// correlation is required (mirrors the `weak_crypto`/`insecure_deser`
/// always-unsafe tables).
const ALWAYS_FLAG_SINKS: &[NoSqlSink] = &[
    NoSqlSink {
        regex: r"\$where\s*:\s*`[^`\n]*\$\{[^`\n]*`",
        label: "$where given a JS template literal with ${...} interpolation",
        fix: "never build $where from an interpolated/concatenated string; disable server-side \
              JavaScript entirely (MongoDB `javascriptEnabled: false`) or replace $where with an \
              equivalent non-JS query operator",
    },
    NoSqlSink {
        regex: r#"\$where\s*:\s*["'][^"'\n]*["']\s*\+"#,
        label: "$where given a string built via concatenation (+) with a variable",
        fix: "never build $where from an interpolated/concatenated string; disable server-side \
              JavaScript entirely (MongoDB `javascriptEnabled: false`) or replace $where with an \
              equivalent non-JS query operator",
    },
    NoSqlSink {
        regex: r#"["']\$where["']\s*:\s*f["']"#,
        label: "pymongo $where given a Python f-string (server-side JS eval with interpolated data)",
        fix: "never build $where from an interpolated/concatenated string; disable server-side \
              JavaScript entirely (MongoDB `javascriptEnabled: false`) or replace $where with an \
              equivalent non-JS query operator",
    },
    NoSqlSink {
        regex: r#"["']\$where["']\s*:\s*["'][^"'\n]*["']\s*\+"#,
        label: "pymongo $where given a string built via concatenation (+) with a variable",
        fix: "never build $where from an interpolated/concatenated string; disable server-side \
              JavaScript entirely (MongoDB `javascriptEnabled: false`) or replace $where with an \
              equivalent non-JS query operator",
    },
    NoSqlSink {
        regex: r"\.(?:find|findOne|findOneAndUpdate|replaceOne|updateOne|updateMany|deleteOne|deleteMany|remove|count|aggregate)\s*\(\s*req\.(?:body|query|params)\s*[,)]",
        label: "raw req.body/req.query/req.params passed directly as a Mongo query filter (operator injection)",
        fix: "reject object/array inputs where a string is expected: validate/cast each field of \
              req.body/req.query/req.params before using it in a query filter, and never pass the \
              whole request object into a query method",
    },
    NoSqlSink {
        regex: r"\.(?:find|find_one|update_one|update_many|delete_one|delete_many|count_documents)\s*\(\s*request\.(?:json\b|get_json\s*\(\s*\)|args\b|form\b)",
        label: "raw request.json/request.get_json()/request.args/request.form passed directly as a pymongo query filter (operator injection)",
        fix: "reject object/array inputs where a string is expected: validate/cast each field of \
              request.json/request.args/request.form before using it in a query filter, and never \
              pass the whole request object into a query method",
    },
    NoSqlSink {
        regex: r"JSON\.parse\s*\(\s*req\.(?:body|query|params)\b",
        label: "JSON.parse(req.body/req.query/req.params) used to build a query object",
        fix: "do not JSON.parse(...) raw request text directly into a query object; validate and \
              allowlist the resulting fields (or use a schema validator) before querying with it",
    },
    NoSqlSink {
        regex: r"\$regex\s*:\s*req\.(?:body|query|params)\.[A-Za-z0-9_]+",
        label: "$regex given a bare, un-cast req.body/req.query/req.params property",
        fix: "validate/escape the value before using it as a $regex pattern (reject non-string \
              types and escape regex metacharacters), or use a fixed/allowlisted pattern instead \
              of raw user input",
    },
];

/// `CYBER-NOSQL-INJECT.1` — NoSQL (MongoDB-family) injection detector:
/// unsanitized request data flowing into `$where`, a raw query-filter
/// argument, an un-cast filter property, `$regex`, or `JSON.parse(req...)`.
#[derive(Debug)]
pub struct NoSqlInjectionValidator {
    rule_id: RuleId,
    always_flag: Vec<RemediationPattern>,
    /// A Mongo query-method call opening paren on the line — scopes the
    /// context-dependent "bare property" check below to lines that are
    /// actually shaping a query filter (as opposed to, say, a log line that
    /// happens to read a request property).
    mongo_call_js: Regex,
    /// `: req.body.<prop>` / `: req.query.<prop>` / `: req.params.<prop>`
    /// with NOTHING but the colon and optional whitespace between the key
    /// and the bare request token. Any wrapping call (`String(`, `Number(`,
    /// `parseInt(`, a mongoose/validator helper, ...) puts different text
    /// immediately after the colon, so this regex naturally does not match
    /// a cast/validated value — no lookaround is needed.
    bare_property_request_value: Regex,
}

impl NoSqlInjectionValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let mut always_flag = Vec::with_capacity(ALWAYS_FLAG_SINKS.len());
        for sink in ALWAYS_FLAG_SINKS {
            always_flag.push(RemediationPattern::compile_source(
                "cyberskillsNoSqlInjectSink",
                sink,
            )?);
        }
        let mongo_call_js = Regex::new(
            r"\.(?:find|findOne|findOneAndUpdate|replaceOne|updateOne|updateMany|deleteOne|deleteMany|remove|count|aggregate)\s*\(",
        )
        .map_err(|err| crate::boundary::regex::decode("cyberskillsNoSqlInjectMongoCallJs", err))?;
        let bare_property_request_value =
            Regex::new(r":\s*req\.(?:body|query|params)\.[A-Za-z0-9_]+").map_err(|err| {
                crate::boundary::regex::decode(
                    "cyberskillsNoSqlInjectBarePropertyRequestValue",
                    err,
                )
            })?;
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberNosqlInject.id(),
            always_flag,
            mongo_call_js,
            bare_property_request_value,
        })
    }
}

impl Validator for NoSqlInjectionValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (index, line) in input.source.as_str().lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);

            let mut matched_labels: Vec<&str> = Vec::new();
            let mut matched_fixes: Vec<&str> = Vec::new();
            for pattern in &self.always_flag {
                if pattern.regex().is_match(line)
                    && !matched_labels.contains(&pattern.label().as_str())
                {
                    matched_labels.push(pattern.label().as_str());
                    matched_fixes.push(pattern.fix().as_str());
                }
            }
            if !matched_labels.is_empty() {
                findings.extend(crate::boundary::finding::from_source(
                    (&self.rule_id, Severity::Error),
                    "Unsanitized request data flows into a MongoDB query",
                    format!(
                        "Line builds a MongoDB query from unsanitized request data: {}. Fix: {}.",
                        matched_labels.join(", "),
                        matched_fixes.join("; ")
                    ),
                    input.file,
                    (line_number, Some(line)),
                ));
            }

            // Property-level operator injection: a Mongo query method is
            // called on this line AND one of its filter properties is a
            // bare (un-cast) req.body/req.query/req.params value.
            if self.mongo_call_js.is_match(line) && self.bare_property_request_value.is_match(line)
            {
                findings.extend(crate::boundary::finding::from_source(
                    (&self.rule_id, Severity::Error),
                    "Unsanitized request data flows into a MongoDB query",
                    "Line passes an un-cast req.body/req.query/req.params property \
                             directly as a query-filter value inside a Mongo query method call. \
                             An attacker can send an object instead of a string (e.g. \
                             {\"$gt\": \"\"} or {\"$ne\": null}) to change the query's meaning \
                             and bypass a check or match unintended documents. Fix: cast the \
                             value with String(...)/Number(...)/parseInt(...) (or otherwise \
                             validate its type) before using it in a query filter.",
                    input.file,
                    (line_number, Some(line)),
                ));
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use crate::boundary::fixture::run_manifest_fixture_parity;

    use super::NoSqlInjectionValidator;

    #[test]
    fn cyberskills_nosql_injection() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoSqlInjectionValidator::new()?;
        run_manifest_fixture_parity(
            &validator,
            "tests/fixtures/cyberskills/web.nosql-injection/bad/vuln.js",
            "tests/fixtures/cyberskills/web.nosql-injection/good/safe.js",
        )?;
        Ok(())
    }
}
