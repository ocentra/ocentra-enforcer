use crate::utils::json_string;
use crate::{Finding, ScanReport};

impl Finding {
    pub(crate) fn stable_key(&self) -> String {
        format!(
            "{}:{:010}:{:010}:{}:{}:{}",
            self.file,
            self.line,
            self.column,
            self.category.as_str(),
            self.literal_hash,
            self.rule_id
        )
    }

    fn to_json(&self) -> String {
        format!(
            concat!(
                "{{",
                "\"ruleId\":{},",
                "\"severity\":{},",
                "\"file\":{},",
                "\"line\":{},",
                "\"column\":{},",
                "\"language\":{},",
                "\"fileRole\":{},",
                "\"literalKind\":{},",
                "\"literalPreview\":{},",
                "\"literalHash\":{},",
                "\"category\":{},",
                "\"score\":{},",
                "\"confidence\":{},",
                "\"blocking\":{},",
                "\"reason\":{},",
                "\"suggestion\":{},",
                "\"context\":{}",
                "}}"
            ),
            json_string(&self.rule_id),
            json_string(&self.severity),
            json_string(&self.file),
            self.line,
            self.column,
            json_string(&self.language),
            json_string(self.file_role.as_str()),
            json_string(self.literal_kind.as_str()),
            json_string(&self.literal_preview),
            json_string(&self.literal_hash),
            json_string(self.category.as_str()),
            self.score,
            json_string(&self.confidence),
            self.blocking,
            json_string(&self.reason),
            json_string(&self.suggestion),
            json_string(&self.context),
        )
    }
}

impl ScanReport {
    pub fn to_json_pretty(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str(&format!("  \"ok\": {},\n", self.ok));
        out.push_str("  \"summary\": {");
        out.push_str(&format!(
            "\n    \"filesDiscovered\": {},\n    \"filesScanned\": {},\n    \"filesIgnored\": {},\n    \"literalsFound\": {},\n    \"literalRisks\": {},\n    \"hardFindings\": {},\n    \"durationMs\": {}\n  }},\n",
            self.summary.files_discovered,
            self.summary.files_scanned,
            self.summary.files_ignored,
            self.summary.literals_found,
            self.summary.literal_risks,
            self.summary.hard_findings,
            self.summary.duration_ms
        ));
        out.push_str("  \"ignored\": {");
        out.push_str(&format!(
            "\n    \"gitignore\": {},\n    \"defaultDirs\": {},\n    \"defaultFiles\": {},\n    \"binary\": {},\n    \"tooLarge\": {},\n    \"unknownLanguage\": {}\n  }},\n",
            self.ignored.gitignore,
            self.ignored.default_dirs,
            self.ignored.default_files,
            self.ignored.binary,
            self.ignored.too_large,
            self.ignored.unknown_language
        ));
        write_language_counts(&mut out, self);
        write_finding_list(&mut out, "hardFindings", &self.hard_findings, true);
        write_finding_list(&mut out, "literalRisks", &self.literal_risks, false);
        out.push_str("}\n");
        out
    }

    pub fn to_json_lines(&self) -> Vec<String> {
        self.hard_findings
            .iter()
            .chain(self.literal_risks.iter())
            .map(Finding::to_json)
            .collect()
    }

    pub fn to_human(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "Ocentra Literal Scan: {}\nfiles scanned: {}, literals: {}, hard findings: {}, literal risks: {}\n",
            if self.ok { "PASS" } else { "FAIL" },
            self.summary.files_scanned,
            self.summary.literals_found,
            self.summary.hard_findings,
            self.summary.literal_risks
        ));
        for finding in self.hard_findings.iter().chain(self.literal_risks.iter()) {
            out.push_str(&format!(
                "\n{}:{}:{} {} score={} {}\n  {}\n  {}\n  {}\n",
                finding.file,
                finding.line,
                finding.column,
                finding.rule_id,
                finding.score,
                finding.category.as_str(),
                finding.literal_preview,
                finding.reason,
                finding.suggestion
            ));
        }
        out
    }
}

fn write_language_counts(out: &mut String, report: &ScanReport) {
    out.push_str("  \"languages\": {");
    if report.languages.is_empty() {
        out.push_str("},\n");
        return;
    }
    out.push('\n');
    for (idx, (language, count)) in report.languages.iter().enumerate() {
        let comma = if idx + 1 == report.languages.len() {
            ""
        } else {
            ","
        };
        out.push_str(&format!(
            "    {}: {}{}\n",
            json_string(language),
            count,
            comma
        ));
    }
    out.push_str("  },\n");
}

fn write_finding_list(out: &mut String, label: &str, findings: &[Finding], trailing_comma: bool) {
    out.push_str(&format!("  \"{label}\": ["));
    if findings.is_empty() {
        out.push_str(if trailing_comma { "],\n" } else { "]\n" });
        return;
    }
    out.push('\n');
    for (idx, finding) in findings.iter().enumerate() {
        let comma = if idx + 1 == findings.len() { "" } else { "," };
        out.push_str(&format!("    {}{}\n", finding.to_json(), comma));
    }
    out.push_str(if trailing_comma { "  ],\n" } else { "  ]\n" });
}
