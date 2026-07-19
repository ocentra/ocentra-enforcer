//! `CYBER-MCP-POISON.1` — harvest target:
//! `vendor/anthropic-cybersecurity-skills/skills/auditing-mcp-servers-for-tool-poisoning/`
//! (`SKILL.md` Workflow step 2 "Inspect raw tool descriptions" and
//! `scripts/agent.py` `POISON_PATTERNS` + `SMUGGLE`, L25-34). A malicious
//! MCP server can embed hidden instructions in a tool's `description` field
//! (OWASP MCP03:2025 tool poisoning; MITRE ATLAS AML.T0010 ML Supply Chain
//! Compromise / AML.T0051.001 indirect prompt injection): the agent's model
//! reads that description as trusted context before deciding whether to call
//! the tool, and a capable model will silently follow embedded directives
//! while returning a normal-looking response to the user.
//!
//! This validator is a 1:1 port of the vendor agent's two heuristics, applied
//! to any text/JSON source (MCP tool manifests, tool description strings) line
//! by line — NOT a broadened reimplementation. The vendor's seven
//! `POISON_PATTERNS` (matched case-insensitively, as the vendor lowercases the
//! description first) are:
//!
//! 1. `do not (tell|inform|mention to) the user` — concealment directive.
//! 2. `ignore (previous|prior|all) instructions` — instruction-override.
//! 3. `<important>|<system>|[system]` — pseudo-tag smuggling a counterfeit system
//!    prompt into the description.
//! 4. `read .*(\.ssh|id_rsa|\.env|credentials|passwd)` — directive to read a
//!    sensitive credential/key file.
//! 5. `(send|exfiltrate|post) .* to https?://` — directive to exfiltrate data
//!    to a remote URL.
//! 6. `before (using|calling) (this|any) tool,? (you must|always)` — a
//!    preamble directive that hijacks the agent before every tool call. (This
//!    is the ONLY context in which a bare "you must" is treated as a red flag;
//!    a standalone "you must" — e.g. "you must provide an API key" — is a
//!    legitimate tool-usage note and is deliberately NOT matched.)
//! 7. `call (the )?\w+ tool (first|before)` — a directive chaining the agent
//!    into calling another (attacker-chosen) tool.
//!
//! plus the vendor `SMUGGLE` character class — hidden/invisible/bidi Unicode
//! smuggled into the text: U+200B-U+200F (zero-width + directional marks),
//! U+202A-U+202E (bidi overrides), U+2060-U+206F (invisible formatting), and
//! U+E0000-U+E007F (Unicode TAG characters). These are invisible to a human
//! reviewing the tool description but present in the text an LLM reads;
//! detected by iterating `char`s directly, since these codepoints have no
//! reliable representation in a portable text regex.

use crate::boundary::pattern::{LabelledPattern, LabelledPatternSource as PoisonPattern};
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The vendor agent's seven `POISON_PATTERNS` (agent.py L25-33), ported
/// verbatim in meaning (case-insensitive via the inline `(?i)` flag, matching
/// the vendor's `description.lower()` pre-pass). No pattern is broadened.
const POISON_PATTERNS_SRC: &[PoisonPattern] = &[
    PoisonPattern {
        regex: r"(?i)do not (?:tell|inform|mention to) the user",
        label: "concealment directive (\"do not tell/inform/mention to the user\")",
    },
    PoisonPattern {
        regex: r"(?i)ignore (?:previous|prior|all) instructions",
        label: "instruction-override directive (\"ignore previous/prior/all instructions\")",
    },
    PoisonPattern {
        regex: r"(?i)<important>|<system>|\[system\]",
        label: "pseudo-tag smuggling a fake system prompt (<important>/<system>/[system])",
    },
    PoisonPattern {
        regex: r"(?i)read .*(?:\.ssh|id_rsa|\.env|credentials|passwd)",
        label: "directive to read a sensitive credential/key file",
    },
    PoisonPattern {
        regex: r"(?i)(?:send|exfiltrate|post) .* to https?://",
        label: "directive to exfiltrate data to a remote URL",
    },
    PoisonPattern {
        regex: r"(?i)before (?:using|calling) (?:this|any) tool,? (?:you must|always)",
        label: "preamble directive that hijacks the agent before a tool call",
    },
    PoisonPattern {
        regex: r"(?i)call (?:the )?\w+ tool (?:first|before)",
        label: "directive chaining the agent into calling another tool",
    },
];

/// The vendor `SMUGGLE` character class (agent.py L34): zero-width,
/// directional-mark, bidi-override, invisible-formatting, and Unicode-TAG
/// codepoints — the classic tool-poisoning smuggling vector (a human reviewer
/// sees nothing; an LLM still reads the codepoint as part of the description).
/// Names which smuggling category a suspicious codepoint belongs to, for the
/// finding detail.
/// `CYBER-MCP-POISON.1` — flags MCP tool-poisoning / prompt-injection markers
/// in MCP server tool definitions: instruction-override / concealment /
/// exfiltration / tool-chaining directives, and hidden/bidi Unicode smuggled
/// into tool metadata.
#[derive(Debug)]
pub struct McpToolPoisoningValidator {
    rule_id: RuleId,
    poison_patterns: Vec<LabelledPattern>,
}

impl McpToolPoisoningValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let mut poison_patterns = Vec::with_capacity(POISON_PATTERNS_SRC.len());
        for entry in POISON_PATTERNS_SRC {
            poison_patterns.push(LabelledPattern::compile_source(
                "cyberskillsMcpToolPoisoningPattern",
                entry,
            )?);
        }
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberMcpPoison.id(),
            poison_patterns,
        })
    }
}

impl Validator for McpToolPoisoningValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (index, line) in input.source.as_str().lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);

            let mut directive_labels: Vec<&str> = Vec::new();
            for pattern in &self.poison_patterns {
                if pattern.regex().is_match(line)
                    && !directive_labels.contains(&pattern.label().as_str())
                {
                    directive_labels.push(pattern.label().as_str());
                }
            }
            if !directive_labels.is_empty() {
                findings.extend(crate::boundary::finding::from_source(
                    (&self.rule_id, Severity::Error),
                    "Tool-poisoning directive embedded in MCP tool metadata",
                    format!(
                        "Line carries a directive aimed at the agent rather than documenting the \
                         tool: {}. MCP tool descriptions are loaded into the agent's context and \
                         treated as trusted input, so this is a tool-poisoning payload (OWASP \
                         MCP03:2025 / MITRE ATLAS AML.T0010). Fix: remove the embedded directive; \
                         a tool description must only describe what the tool does.",
                        directive_labels.join(", ")
                    ),
                    input.file,
                    (line_number, Some(line)),
                ));
            }

            let mut unicode_labels: Vec<&str> = Vec::new();
            for c in line.chars() {
                if crate::boundary::source_predicates::is_suspicious_codepoint(c) {
                    let label = crate::boundary::source_predicates::suspicious_codepoint_label(c);
                    if !unicode_labels.contains(&label) {
                        unicode_labels.push(label);
                    }
                }
            }
            if !unicode_labels.is_empty() {
                findings.extend(crate::boundary::finding::from_source(
                    (&self.rule_id, Severity::Error),
                    "Hidden Unicode smuggled into MCP tool metadata",
                    format!(
                        "Line contains {}, invisible to a human reviewer but present in the text \
                         an LLM reads. This is a classic tool-poisoning smuggling vector for hiding \
                         instructions inside an otherwise innocuous-looking tool description. Fix: \
                         strip non-printing/bidi/tag Unicode codepoints from tool metadata before \
                         loading it into the agent's context.",
                        unicode_labels.join(", ")
                    ),
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

    use super::McpToolPoisoningValidator;

    #[test]
    fn cyberskills_mcp_tool_poisoning() -> Result<(), Box<dyn std::error::Error>> {
        let v = McpToolPoisoningValidator::new()?;
        run_manifest_fixture_parity(
            &v,
            "tests/fixtures/cyberskills/ai.mcp-tool-poisoning/bad/tools.json",
            "tests/fixtures/cyberskills/ai.mcp-tool-poisoning/good/tools.json",
        )?;
        Ok(())
    }
}
