//! `CYBER-WEBSOCKET.1` (h11) — harvest target:
//! `vendor/anthropic-cybersecurity-skills/skills/exploiting-websocket-vulnerabilities/SKILL.md`
//! Step 3 "Test Cross-Site WebSocket Hijacking (CSWSH)" ("Check if the
//! WebSocket handshake is vulnerable to cross-site attacks... If 101
//! Switching Protocols: Origin not validated (vulnerable to CSWSH)"), the
//! "Key Concepts" table rows **CSWSH** ("exploiting missing Origin
//! validation to hijack sessions") and **Origin Validation** ("Server-side
//! check that the WebSocket upgrade request comes from a trusted origin"),
//! Scenario 1 ("A real-time chat application validates the user's cookie
//! during the WebSocket handshake but does not check the Origin header..."),
//! and the Output Format Recommendation list, item 1: "Validate the Origin
//! header during WebSocket handshake."
//!
//! Harvest note: like `super::web_cors`, this vendor skill has no
//! `scripts/agent.py` (or any other script) that contains a static
//! source-code predicate — its `agent.py` (`test_origin_validation`,
//! `test_no_auth_connect`, `test_message_injection`, ...) is a live-target
//! penetration-testing tool that sends real WebSocket upgrade requests
//! (with hostile `Origin` headers) at an already-running server and
//! inspects the HTTP response code; there is nothing in it to port
//! verbatim into a static-source check. Per the h11 workpack fallback, this
//! validator instead narrows the CSWSH root cause the skill names — a
//! WebSocket/socket.io server that never actually validates the handshake's
//! Origin — to the concrete, deterministic SERVER-CONFIGURATION shapes that
//! make a real deployment behave exactly like the vendor's "101 Switching
//! Protocols" (Origin not validated) case, across the two dominant Node.js
//! WebSocket libraries:
//!
//! 1. **socket.io permissive CORS** — `cors: { origin: "*" }` /
//!    `cors: { origin: true }` (socket.io v3+, `cors.origin` reflects/allows
//!    any origin), and the socket.io v2 predecessor
//!    `origins('*:*')` / `origins: '*:*'` (allows any origin AND any port).
//! 2. **Explicit origin-check disabling** — `checkOrigin: false` /
//!    `verifyClient: false` on a raw `ws`/`WebSocket.Server`, which turns
//!    off the library's own client-verification hook entirely.
//! 3. **A `verifyClient` hook that unconditionally accepts every origin** —
//!    `verifyClient: () => true`, `verifyClient: (info, cb) => cb(true)`,
//!    and `verifyClient: function (info, cb) { cb(true) }`: each is an
//!    Origin gate that returns/calls back `true` no matter what the
//!    handshake's Origin header actually is, i.e. no gate at all.
//!
//! Deliberately NOT flagged (false-positive budget): a `verifyClient` whose
//! body actually inspects `info.origin` / `info.req.headers.origin` against
//! an allowlist before ever calling back `true` (real Origin validation —
//! the vendor's own recommended fix); an allowlisted `cors.origin`
//! (string/array of real origins) or a real `origins: 'host:port'` value; a
//! subdomain-glob origin (`*.example.com`, not a literal bare wildcard); a
//! bare `new WebSocket.Server({ port })` with no `verifyClient`/
//! `checkOrigin` key at all (mere absence of a hook is not itself flagged —
//! it would be too noisy, per the h11 false-positive budget); and
//! client-side `new WebSocket("wss://...")` connections, which are not
//! server configuration and are out of this rule's scope. The
//! `verifyClient: function (...) { ... }` check requires the function body
//! to consist of ONLY `cb(true)` (plus whitespace/an optional trailing
//! `;`) — the moment real content (an `if`, a variable read, another
//! statement) sits between the opening `{` and `cb(true)`, the pattern
//! stops matching, which is exactly what keeps a real
//! `if (allowed.includes(info.origin)) { cb(true) } else { cb(false) }`
//! guard clean.
//!
//! Window note: real socket.io configuration is routinely formatted with
//! `cors:` and `origin:` (or a `verifyClient` function's `{`/`cb(true)`/`}`)
//! on separate source lines. The `regex` crate has no lookaround, but its
//! character classes (`[^}]`, `\s`) already cross real newline characters
//! by default (only the `.` metacharacter needs `(?s)` for that), so each
//! pattern below is matched against a small forward-looking window of
//! joined lines (see [`WINDOW_LINES`]) rather than a single line in
//! isolation — the same one-object-literal-at-a-time bound the
//! `[^}]`-delimited patterns already rely on. Each pattern is recorded at
//! most once per file (at the earliest window it matches in), so a
//! multi-line shape does not produce one duplicate `Finding` per
//! overlapping window position.

use crate::boundary::pattern::{LabelledPattern, LabelledPatternSource as WebSocketPattern};
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// How many source lines are folded into one match window (see the "Window
/// note" above). Large enough to cover a `cors: { ... }` object or a small
/// `verifyClient` function body split across a handful of lines, small
/// enough that unrelated, far-apart code never lands in the same window.
const WINDOW_LINES: usize = 5;

/// The insecure server-configuration shapes named in the module doc
/// comment above.
const WEBSOCKET_PATTERNS_SRC: &[WebSocketPattern] = &[
    // 1. socket.io permissive CORS: `cors: { origin: "*" }` / `cors: { origin: true }`.
    WebSocketPattern {
        regex: r#"(?i)cors\s*:\s*\{[^}]*origin\s*:\s*(?:"\*"|'\*'|true\b)"#,
        label:
            "socket.io `cors.origin` is set to a bare wildcard (`\"*\"`/`'*'`) or `true`, so any \
                site can open an authenticated WebSocket connection (CSWSH)",
    },
    // 1b. socket.io v2 predecessor: `origins('*:*')` / `origins: '*:*'`.
    WebSocketPattern {
        regex: r#"(?i)origins\s*(?:\(\s*|:\s*)["']\*:\*["']"#,
        label:
            "socket.io v2 `origins('*:*')`/`origins: '*:*'` accepts every origin and every port \
                (CSWSH)",
    },
    // 2. Explicit origin-check disabling.
    WebSocketPattern {
        regex: r"\bcheckOrigin\s*:\s*false\b",
        label: "`checkOrigin: false` explicitly disables WebSocket Origin validation",
    },
    WebSocketPattern {
        regex: r"\bverifyClient\s*:\s*false\b",
        label: "`verifyClient: false` disables the handshake's client-verification hook entirely",
    },
    // 3. verifyClient that unconditionally accepts any origin.
    WebSocketPattern {
        regex: r"verifyClient\s*:\s*\(\s*\)\s*=>\s*true\b",
        label: "`verifyClient: () => true` accepts every handshake regardless of Origin",
    },
    WebSocketPattern {
        regex: r"verifyClient\s*:\s*\([^)]*\)\s*=>\s*cb\(\s*true\s*\)",
        label: "`verifyClient: (info, cb) => cb(true)` unconditionally calls back true without \
                inspecting the Origin",
    },
    WebSocketPattern {
        regex: r"verifyClient\s*:\s*function\s*\([^)]*\)\s*\{\s*cb\(\s*true\s*\)\s*;?\s*\}",
        label: "`verifyClient: function (info, cb) { cb(true) }` unconditionally calls back true \
                without inspecting the Origin",
    },
];

/// `CYBER-WEBSOCKET.1` — flags insecure WebSocket server configuration that
/// enables cross-site WebSocket hijacking (CSWSH): a permissive socket.io
/// CORS/origins setting, an explicitly disabled origin check, or a
/// `verifyClient` hook that unconditionally accepts every origin.
#[derive(Debug)]
pub struct WebSocketSecurityValidator {
    rule_id: RuleId,
    patterns: Vec<LabelledPattern>,
}

impl WebSocketSecurityValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let mut patterns = Vec::with_capacity(WEBSOCKET_PATTERNS_SRC.len());
        for entry in WEBSOCKET_PATTERNS_SRC {
            patterns.push(LabelledPattern::compile_source(
                "cyberskillsWebSocketSecurityPattern",
                entry,
            )?);
        }
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberWebsocket.id(),
            patterns,
        })
    }
}

impl Validator for WebSocketSecurityValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let lines: Vec<&str> = input.source.as_str().lines().collect();
        let mut findings = Vec::new();
        // Tracks which pattern labels have already produced a Finding
        // anywhere in this file, so a multi-line shape caught by several
        // overlapping windows is reported once, at its earliest window.
        let mut already_flagged: Vec<&str> = Vec::new();

        for (index, line) in lines.iter().copied().enumerate() {
            let window_end = (index + WINDOW_LINES).min(lines.len());
            let Some(window_lines) = lines.get(index..window_end) else {
                continue;
            };
            let window = window_lines.join("\n");
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);

            let mut matched_labels: Vec<&str> = Vec::new();
            for pattern in &self.patterns {
                if !already_flagged.contains(&pattern.label().as_str())
                    && pattern.regex().is_match(&window)
                {
                    matched_labels.push(pattern.label().as_str());
                }
            }

            if !matched_labels.is_empty() {
                for label in &matched_labels {
                    already_flagged.push(*label);
                }
                findings.extend(crate::boundary::finding::from_source(
                    (&self.rule_id, Severity::Error),
                    "Insecure WebSocket server configuration enables cross-site WebSocket \
                     hijacking (CSWSH)",
                    format!(
                        "{}. Fix: validate the WebSocket handshake's Origin header (or socket.io's \
                         `cors.origin`) against an explicit allowlist of trusted origins instead of \
                         accepting any/all origins.",
                        matched_labels.join(", ")
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

    use super::WebSocketSecurityValidator;

    #[test]
    fn cyberskills_websocket_security() -> Result<(), Box<dyn std::error::Error>> {
        let v = WebSocketSecurityValidator::new()?;
        run_manifest_fixture_parity(
            &v,
            "tests/fixtures/cyberskills/web.websocket-security/bad/vuln.js",
            "tests/fixtures/cyberskills/web.websocket-security/good/safe.js",
        )?;
        Ok(())
    }
}
