//! g08 explorer — the self-contained served-HTML view.
//!
//! This is the concrete human-browsable surface g01 mounts under the
//! `"explorer"` slug (see [`crate::serve::VIEW_MOUNTS`]). It projects the
//! typed [`super::ExplorerPayload`] into ONE self-contained HTML document:
//! inline CSS, no `<script>`, and — critically for the acceptance
//! contract — NO external asset of any kind (no `<link>`, no `src=`, no
//! `http(s)://`, no `url(...)`, no remote font/image). The page is valid
//! offline and inside a locked-down Tauri/webview shell.
//!
//! # What a human sees per rule
//! The named rule id, its tier, its rule-family category, a concise
//! explanation, the doctrine-vs-hard-enforcement classification (projected
//! from the tier, see [`super::EnforcementKind`]), and clickable
//! detail/proof links (the doc anchor + the fail/pass fixtures). Both the
//! human-verbose and AI-dense forms render side by side (dual-pane), each
//! projected from the same typed record. A record with a gap renders with
//! an explicit `INCOMPLETE` marker, never as a silently-blank row.
//!
//! # Presentation only
//! Every string shown here is built in Rust from the typed payload; this
//! module chooses layout, not data. It HTML-escapes every interpolated
//! value ([`escape`]) so rule/skill content can never break the document
//! or inject markup.

use std::path::Path;

use enforcer_rules::registry::RuleRegistry;

use super::{render_explorer, CompletenessFlag, ExplorerPayload, RuleEntry, RunMode, SkillEntry};

/// HTML-escape a value for safe interpolation into element text OR a
/// double-quoted attribute. Deliberately conservative (also escapes `'`)
/// so the same helper is safe in every position this module emits.
#[must_use]
pub fn escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Human label for a completeness gap, shown in the `INCOMPLETE` marker.
#[must_use]
fn flag_label(flag: CompletenessFlag) -> &'static str {
    match flag {
        CompletenessFlag::MissingDocAnchor => "missing doc-anchor",
        CompletenessFlag::MissingFixtures => "missing fail/pass fixtures",
    }
}

/// Render one clickable link, or an inert `INCOMPLETE`-style span when the
/// target is empty — the explorer never emits a dead `href=""`.
fn link(label: &str, href: &str) -> String {
    if href.trim().is_empty() {
        format!("<span class=\"link missing\">{}: —</span>", escape(label))
    } else {
        format!(
            "<a class=\"link\" href=\"{}\">{}</a>",
            escape(href),
            escape(label)
        )
    }
}

/// Render one rule as a browse card: header (id + badges), optional
/// `INCOMPLETE` marker, dual-pane verbose/dense, and detail/proof links.
#[must_use]
pub fn render_rule_card(entry: &RuleEntry) -> String {
    let enforcement_class = if entry.enforcement.is_hard_enforcement() {
        "hard"
    } else {
        "doctrine"
    };
    let category = if entry.category.trim().is_empty() {
        "uncategorized".to_owned()
    } else {
        entry.category.clone()
    };

    let mut card = String::new();
    let incomplete_attr = if entry.flags.is_empty() {
        ""
    } else {
        " incomplete"
    };
    card.push_str(&format!("<article class=\"rule{incomplete_attr}\">"));

    // Header: named rule id + severity(tier)/category + doctrine-vs-hard.
    card.push_str("<header class=\"rule-head\">");
    card.push_str(&format!(
        "<span class=\"rid\">{}</span>",
        escape(&entry.rule_id)
    ));
    card.push_str(&format!(
        "<span class=\"badge tier\">tier {}</span>",
        escape(&entry.tier)
    ));
    card.push_str(&format!(
        "<span class=\"badge cat\">{}</span>",
        escape(&category)
    ));
    card.push_str(&format!(
        "<span class=\"badge {enforcement_class}\">{}</span>",
        escape(entry.enforcement.axis_label())
    ));
    card.push_str(&format!(
        "<span class=\"badge fw\">{}</span>",
        escape(&entry.framework)
    ));
    card.push_str("</header>");

    // Completeness: never a silent blank — an explicit flagged marker.
    if !entry.flags.is_empty() {
        let names = entry
            .flags
            .iter()
            .map(|f| flag_label(*f))
            .collect::<Vec<_>>()
            .join(", ");
        card.push_str(&format!(
            "<p class=\"incomplete-marker\">INCOMPLETE: {}</p>",
            escape(&names)
        ));
    }

    // Dual-pane: human-verbose beside AI-dense, both from the typed record.
    card.push_str("<div class=\"panes\">");
    card.push_str("<section class=\"pane verbose\"><h4>Human</h4>");
    card.push_str(&format!(
        "<p class=\"title\">{}</p>",
        escape(&entry.verbose.title)
    ));
    card.push_str(&format!(
        "<p class=\"why\">{}</p>",
        escape(&entry.verbose.why_it_matters)
    ));
    card.push_str(&format!(
        "<p class=\"ex fail\"><span>fail</span> {}</p>",
        escape(&entry.verbose.fail_example)
    ));
    card.push_str(&format!(
        "<p class=\"ex pass\"><span>pass</span> {}</p>",
        escape(&entry.verbose.pass_example)
    ));
    card.push_str("</section>");
    card.push_str("<section class=\"pane dense\"><h4>AI-dense</h4>");
    card.push_str(&format!("<pre>{}</pre>", escape(&entry.dense.summary)));
    card.push_str(&format!("<pre>{}</pre>", escape(&entry.dense.fixtures)));
    card.push_str("</section>");
    card.push_str("</div>");

    // Detail + proof links.
    card.push_str("<footer class=\"links\">");
    card.push_str(&link("detail", &entry.links.detail));
    card.push_str(&link("proof: fail", &entry.links.proof_fail));
    card.push_str(&link("proof: pass", &entry.links.proof_pass));
    card.push_str("</footer>");

    card.push_str("</article>");
    card
}

/// Render one skill as a browse card: its name + source path, and its own
/// dual-audience split (dense fenced block beside the verbose prose).
#[must_use]
pub fn render_skill_card(skill: &SkillEntry) -> String {
    let mut card = String::new();
    card.push_str("<article class=\"skill\">");
    card.push_str("<header class=\"rule-head\">");
    card.push_str(&format!(
        "<span class=\"rid\">{}</span>",
        escape(&skill.name)
    ));
    card.push_str(&format!(
        "<span class=\"badge fw\">{}</span>",
        escape(&skill.source_path)
    ));
    card.push_str("</header>");
    card.push_str("<div class=\"panes\">");
    card.push_str("<section class=\"pane verbose\"><h4>Human (prose)</h4>");
    card.push_str(&format!("<pre>{}</pre>", escape(&skill.verbose)));
    card.push_str("</section>");
    card.push_str("<section class=\"pane dense\"><h4>AI-dense</h4>");
    let dense = if skill.dense.trim().is_empty() {
        "(this skill carries no ai-dense block)"
    } else {
        &skill.dense
    };
    card.push_str(&format!("<pre>{}</pre>", escape(dense)));
    card.push_str("</section>");
    card.push_str("</div>");
    card.push_str("</article>");
    card
}

/// Inline stylesheet. No `url(...)`, no `@import`, no remote font — fully
/// self-contained so the acceptance "no external asset is fetched"
/// property holds by construction.
const STYLE: &str = "\
body{font-family:system-ui,sans-serif;margin:0;padding:1.5rem;line-height:1.4}\
h1{font-size:1.3rem;margin:0 0 .25rem}\
.legend{font-size:.85rem;opacity:.8;margin:0 0 1rem}\
.count{font-weight:600}\
.rule,.skill{border:1px solid #8884;border-radius:8px;padding:.75rem 1rem;margin:.75rem 0}\
.rule.incomplete{border-color:#c0392b;border-width:2px}\
.rule-head{display:flex;flex-wrap:wrap;gap:.4rem;align-items:center;margin-bottom:.5rem}\
.rid{font-weight:700;font-family:ui-monospace,monospace}\
.badge{font-size:.72rem;padding:.1rem .5rem;border-radius:999px;border:1px solid #8886}\
.badge.hard{background:#c0392b22;border-color:#c0392b}\
.badge.doctrine{background:#2980b922;border-color:#2980b9}\
.badge.tier{background:#8882}\
.incomplete-marker{color:#c0392b;font-weight:700;margin:.25rem 0}\
.panes{display:flex;flex-wrap:wrap;gap:1rem}\
.pane{flex:1 1 18rem;min-width:0}\
.pane h4{margin:.25rem 0;font-size:.78rem;text-transform:uppercase;opacity:.7}\
.pane pre{white-space:pre-wrap;word-break:break-word;background:#8881;padding:.5rem;border-radius:6px;font-size:.8rem}\
.ex span{font-weight:700;font-size:.7rem;text-transform:uppercase;margin-right:.4rem}\
.ex.fail span{color:#c0392b}.ex.pass span{color:#27ae60}\
.links{display:flex;flex-wrap:wrap;gap:.75rem;margin-top:.5rem;font-size:.82rem}\
.link.missing{opacity:.6}\
";

/// Render the full explorer payload into ONE self-contained HTML document.
/// An empty payload (e.g. a [`RunMode::Silent`] build, or an empty
/// registry) renders an explicit empty-state, never a broken page.
#[must_use]
pub fn render_explorer_html(payload: &ExplorerPayload) -> String {
    let mut body = String::new();
    body.push_str(
        "<h1>Rules &amp; Skills Explorer</h1>\
         <p class=\"legend\">\
         <span class=\"badge hard\">hard enforcement</span> = mechanical gate (tier T1/T2) · \
         <span class=\"badge doctrine\">doctrine (advisory)</span> = review-assist (tier T3). \
         Every entry is projected from the typed rule record; a gap renders as \
         <b>INCOMPLETE</b>, never blank.</p>",
    );
    body.push_str(&format!(
        "<p><span class=\"count\">{}</span> rules · \
         <span class=\"count\">{}</span> skills</p>",
        payload.rules.len(),
        payload.skills.len()
    ));

    body.push_str("<section data-view=\"explorer-rules\">");
    if payload.rules.is_empty() {
        body.push_str("<p class=\"legend\">No rules in the registry.</p>");
    }
    for entry in &payload.rules {
        body.push_str(&render_rule_card(entry));
    }
    body.push_str("</section>");

    body.push_str("<section data-view=\"explorer-skills\">");
    if payload.skills.is_empty() {
        body.push_str("<p class=\"legend\">No skills found.</p>");
    }
    for skill in &payload.skills {
        body.push_str(&render_skill_card(skill));
    }
    body.push_str("</section>");

    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>Rules &amp; Skills Explorer</title><style>{STYLE}</style></head>\
         <body data-enforcer-ui-view=\"explorer\">{body}</body></html>"
    )
}

/// Silent-mode-aware entry point: renders the explorer HTML for a human,
/// and NOTHING (the empty string — zero UI output) for a silent inline
/// agent run, honoring the same f04 gate seam as [`render_explorer`].
#[must_use]
pub fn render_explorer_view(mode: RunMode, registry: &RuleRegistry, skills_root: &Path) -> String {
    match mode {
        RunMode::Silent => String::new(),
        RunMode::Human => {
            render_explorer_html(&render_explorer(RunMode::Human, registry, skills_root))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{escape, render_explorer_html, render_explorer_view, render_rule_card};
    use crate::explorer::{render_rule, RunMode};
    use enforcer_domain::severity::Tier;
    use enforcer_rules::registry::{FixtureRef, RuleRecord, RuleRegistry, ValidatorRef};

    fn record(
        rule_id: &str,
        tier: Tier,
    ) -> Result<RuleRecord, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(RuleRecord {
            rule_id: rule_id.parse()?,
            version: 1,
            title: "No re-export barrels".to_owned(),
            tier,
            validator: ValidatorRef {
                crate_name: "enforcer-lang-rust".to_owned(),
                path: "no_reexports::NoReexportsValidator".to_owned(),
            },
            fixtures: FixtureRef {
                fail: "crates/x/fixtures/sample/fail.rs".to_owned(),
                pass: "crates/x/fixtures/sample/pass.rs".to_owned(),
            },
            doc_anchor: "docs/rules/SAMPLE.md#SAMPLE-1".to_owned(),
            tags: vec!["rust".to_owned(), "reexports".to_owned()],
            params: serde_json::Value::Null,
        })
    }

    /// The rendered page references NO external asset — the acceptance
    /// contract's "no external asset is fetched" property, asserted
    /// mechanically over the emitted bytes.
    #[test]
    fn rendered_html_fetches_no_external_asset() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![record("RR-1.1", Tier::T1)?])?;
        let html = render_explorer_view(RunMode::Human, &registry, std::path::Path::new("skills"));
        for forbidden in [
            "http://", "https://", "src=", "<link", "@import", "url(", "//cdn",
        ] {
            assert!(
                !html.contains(forbidden),
                "self-contained HTML must not reference `{forbidden}`"
            );
        }
        assert!(html.starts_with("<!doctype html>"));
        Ok(())
    }

    /// A complete rule card carries EVERY dimension the workpack requires:
    /// the named id, tier, category, the doctrine-vs-hard label, the
    /// concise explanation, BOTH forms, and clickable detail/proof links.
    #[test]
    fn rule_card_shows_every_required_dimension() -> Result<(), Box<dyn std::error::Error>> {
        let entry = render_rule(&record("T1-NOREEXPORT.1", Tier::T1)?);
        let card = render_rule_card(&entry);
        assert!(card.contains("T1-NOREEXPORT.1"), "named rule id");
        assert!(card.contains("tier T1"), "tier/severity");
        assert!(card.contains("rust"), "category from tags");
        assert!(card.contains("hard enforcement"), "doctrine-vs-hard label");
        assert!(card.contains("Human"), "verbose pane");
        assert!(card.contains("AI-dense"), "dense pane");
        assert!(
            card.contains("href=\"docs/rules/SAMPLE.md#SAMPLE-1\""),
            "clickable detail link"
        );
        assert!(
            card.contains("href=\"crates/x/fixtures/sample/fail.rs\""),
            "clickable proof (fail) link"
        );
        Ok(())
    }

    /// A T3 rule renders the DOCTRINE (advisory) badge, not hard
    /// enforcement — the doctrine-vs-hard split is visible in the output.
    #[test]
    fn t3_rule_card_shows_doctrine_not_hard() -> Result<(), Box<dyn std::error::Error>> {
        let entry = render_rule(&record("RR-3.1", Tier::T3)?);
        let card = render_rule_card(&entry);
        assert!(card.contains("doctrine (advisory)"));
        assert!(card.contains("class=\"badge doctrine\""));
        Ok(())
    }

    /// An incomplete rule renders an explicit INCOMPLETE marker with the
    /// gap named — never a silently-blank row.
    #[test]
    fn incomplete_rule_card_is_flagged_visibly() -> Result<(), Box<dyn std::error::Error>> {
        let mut bad = record("RR-9.1", Tier::T1)?;
        bad.doc_anchor = "   ".to_owned();
        let entry = render_rule(&bad);
        let card = render_rule_card(&entry);
        assert!(card.contains("INCOMPLETE"));
        assert!(card.contains("missing doc-anchor"));
        assert!(card.contains("class=\"rule incomplete\""));
        // A missing detail target renders inert, never a dead href="".
        assert!(!card.contains("href=\"\""));
        Ok(())
    }

    /// Rule/skill content that itself contains markup is escaped — it can
    /// never break the document or inject an element/asset.
    #[test]
    fn interpolated_markup_is_escaped() -> Result<(), Box<dyn std::error::Error>> {
        let mut evil = record("RR-1.2", Tier::T1)?;
        evil.title = "<script src=https://evil/x.js></script>".to_owned();
        let entry = render_rule(&evil);
        let card = render_rule_card(&entry);
        assert!(!card.contains("<script src=https://evil"));
        assert!(card.contains("&lt;script"));
        Ok(())
    }

    #[test]
    fn escape_covers_all_five_metacharacters() {
        assert_eq!(
            escape("<a& \"b\" 'c'>"),
            "&lt;a&amp; &quot;b&quot; &#x27;c&#x27;&gt;"
        );
    }

    /// Silent mode emits ZERO UI output — the empty string, not a page
    /// with zero cards.
    #[test]
    fn silent_mode_renders_no_html() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![record("RR-1.1", Tier::T1)?])?;
        let html = render_explorer_view(RunMode::Silent, &registry, std::path::Path::new("skills"));
        assert!(html.is_empty());
        Ok(())
    }

    /// An empty registry renders a valid empty-state page, not a panic or
    /// a broken document.
    #[test]
    fn empty_payload_renders_empty_state() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![])?;
        let html = render_explorer_view(RunMode::Human, &registry, std::path::Path::new("skills"));
        assert!(html.contains("No rules in the registry."));
        assert!(html.starts_with("<!doctype html>"));
        Ok(())
    }

    /// End-to-end HTML render carries both a rule and, when present, a
    /// skill; the page is one self-contained document.
    #[test]
    fn full_page_has_both_sections() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![record("RR-1.1", Tier::T1)?])?;
        let payload = crate::explorer::render_explorer(
            RunMode::Human,
            &registry,
            std::path::Path::new("skills"),
        );
        let html = render_explorer_html(&payload);
        assert!(html.contains("data-view=\"explorer-rules\""));
        assert!(html.contains("data-view=\"explorer-skills\""));
        Ok(())
    }
}
