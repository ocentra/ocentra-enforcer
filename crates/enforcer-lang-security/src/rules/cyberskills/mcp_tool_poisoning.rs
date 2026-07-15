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
//! 3. `<important>|<system>|[system]` — pseudo-tag smuggling a fake system
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

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

/// One high-confidence tool-poisoning phrase pattern (a vendor
/// `POISON_PATTERNS` entry), paired with a human-readable label.
struct PoisonPattern {
    regex: &'static str,
    label: &'static str,
}

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
fn is_suspicious_codepoint(c: char) -> bool {
    matches!(
        c as u32,
        0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x206F | 0xE0000..=0xE007F
    )
}

/// Names which smuggling category a suspicious codepoint belongs to, for the
/// finding detail.
fn suspicious_codepoint_label(c: char) -> &'static str {
    match c as u32 {
        0x200B..=0x200F => "a zero-width / directional-mark character (U+200B-U+200F)",
        0x202A..=0x202E => "a bidi-override character (U+202A-U+202E)",
        0x2060..=0x206F => "an invisible formatting character (U+2060-U+206F)",
        _ => "a Unicode TAG character (U+E0000-U+E007F)",
    }
}

/// `CYBER-MCP-POISON.1` — flags MCP tool-poisoning / prompt-injection markers
/// in MCP server tool definitions: instruction-override / concealment /
/// exfiltration / tool-chaining directives, and hidden/bidi Unicode smuggled
/// into tool metadata.
pub struct McpToolPoisoningValidator {
    rule_id: RuleId,
    poison_patterns: Vec<(Regex, &'static str)>,
}

impl McpToolPoisoningValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let mut poison_patterns = Vec::with_capacity(POISON_PATTERNS_SRC.len());
        for entry in POISON_PATTERNS_SRC {
            let regex = Regex::new(entry.regex).map_err(|err| {
                DecodeError::new("cyberskillsMcpToolPoisoningPattern", err.to_string())
            })?;
            poison_patterns.push((regex, entry.label));
        }
        Ok(Self {
            rule_id: "CYBER-MCP-POISON.1".parse()?,
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
        for (index, line) in input.source.lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);

            let mut directive_labels: Vec<&str> = Vec::new();
            for (regex, label) in &self.poison_patterns {
                if regex.is_match(line) && !directive_labels.contains(label) {
                    directive_labels.push(*label);
                }
            }
            if !directive_labels.is_empty() {
                findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "Tool-poisoning directive embedded in MCP tool metadata".to_owned(),
                    detail: format!(
                        "Line carries a directive aimed at the agent rather than documenting the \
                         tool: {}. MCP tool descriptions are loaded into the agent's context and \
                         treated as trusted input, so this is a tool-poisoning payload (OWASP \
                         MCP03:2025 / MITRE ATLAS AML.T0010). Fix: remove the embedded directive; \
                         a tool description must only describe what the tool does.",
                        directive_labels.join(", ")
                    ),
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(line.to_owned()),
                });
            }

            let mut unicode_labels: Vec<&str> = Vec::new();
            for c in line.chars() {
                if is_suspicious_codepoint(c) {
                    let label = suspicious_codepoint_label(c);
                    if !unicode_labels.contains(&label) {
                        unicode_labels.push(label);
                    }
                }
            }
            if !unicode_labels.is_empty() {
                findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "Hidden Unicode smuggled into MCP tool metadata".to_owned(),
                    detail: format!(
                        "Line contains {}, invisible to a human reviewer but present in the text \
                         an LLM reads. This is a classic tool-poisoning smuggling vector for hiding \
                         instructions inside an otherwise innocuous-looking tool description. Fix: \
                         strip non-printing/bidi/tag Unicode codepoints from tool metadata before \
                         loading it into the agent's context.",
                        unicode_labels.join(", ")
                    ),
                    file: input.file.clone(),
                    line: line_number,
                    snippet: Some(line.to_owned()),
                });
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::McpToolPoisoningValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn cyberskills_mcp_tool_poisoning() -> Result<(), Box<dyn std::error::Error>> {
        let v = McpToolPoisoningValidator::new()?;
        run_fixture_parity(
            &v,
            &manifest_dir(),
            "tests/fixtures/cyberskills/ai.mcp-tool-poisoning/bad/tools.json",
            "tests/fixtures/cyberskills/ai.mcp-tool-poisoning/good/tools.json",
        )?;
        Ok(())
    }
}
