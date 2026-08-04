//! Native frozen-MJS parity for configuration lockdown and waiver governance.
//!
//! The normal loader remains fail-closed.  This module consumes the typed
//! diagnostic projection solely to turn invalid settings into reviewable
//! findings, rather than silently defaulting or accepting them.

use enforcer_config::serde::diagnostics::ConfigParseDiagnostics;
use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingSnippet, FindingTitle, Report, ReportOutcome,
    ScanScope, Violation,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;
use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::Path;

#[derive(serde::Deserialize)]
struct Catalog {
    rules: Vec<CatalogRule>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogRule {
    id: String,
    severity: String,
    lock_level: String,
    #[serde(default)]
    can_disable: bool,
    #[serde(default)]
    can_downgrade: bool,
    #[serde(default)]
    waivable: bool,
}

#[derive(Clone, Copy)]
enum CheckKind {
    Lockdown,
    Waiver,
}

/// Validate locked configuration fields against the reviewed catalog.
pub fn check_config_lockdown(path: &Path, root: &Path, scope: ScanScope) -> Result<Report, String> {
    check(path, root, scope, CheckKind::Lockdown)
}
/// Validate project waiver policy against the reviewed catalog.
pub fn check_waiver_policy(path: &Path, root: &Path, scope: ScanScope) -> Result<Report, String> {
    check(path, root, scope, CheckKind::Waiver)
}

fn check(path: &Path, root: &Path, scope: ScanScope, kind: CheckKind) -> Result<Report, String> {
    let diagnostics = enforcer_config::serde::diagnostics::inspect_project_config(path);
    if let Some(error) = diagnostics.load_error.as_deref() {
        return Err(format!(
            "cannot inspect project configuration {}: {error}",
            path.display()
        ));
    }
    let catalog = catalog(root)?;
    let file = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("ocentra-enforcer.config.json");
    let findings = match kind {
        CheckKind::Lockdown => lockdown_findings(&diagnostics, &catalog, file)?,
        CheckKind::Waiver => waiver_findings(&diagnostics, &catalog, file)?,
    };
    report(scope, findings)
}

fn catalog(root: &Path) -> Result<BTreeMap<String, CatalogRule>, String> {
    let text = std::fs::read_to_string(root.join("rules/rules.json"))
        .map_err(|error| format!("cannot read native rule catalog: {error}"))?;
    let parsed: Catalog = serde_json::from_str(&text)
        .map_err(|error| format!("native rule catalog is malformed: {error}"))?;
    Ok(parsed
        .rules
        .into_iter()
        .map(|rule| (rule.id.clone(), rule))
        .collect())
}

fn lockdown_findings(
    d: &ConfigParseDiagnostics,
    catalog: &BTreeMap<String, CatalogRule>,
    file: &str,
) -> Result<Vec<Finding>, String> {
    let mut findings = Vec::new();
    for key in &d.unknown_top_level_keys {
        findings.push(finding(
            "CFG-1.9",
            file,
            format!("unknown config key {key}"),
            Some(key.clone()),
        )?);
    }
    if d.missing_schema_version || d.missing_profile_name {
        findings.push(finding(
            "CFG-1.10",
            file,
            "config must declare schemaVersion and profileName for unambiguous layering".to_owned(),
            None,
        )?);
    }
    if let Some(profile) = &d.profile_name {
        if !["strict", "default", "ocentra-enforcer", "ocentra-parent"].contains(&profile.as_str())
        {
            findings.push(finding(
                "CFG-1.11",
                file,
                format!("unknown profileName {profile}"),
                Some(profile.clone()),
            )?);
        }
    }
    if d.config_change_requires_self_check && !d.policy_integrity_checked {
        findings.push(finding(
            "CFG-1.12",
            file,
            "config change requires policyIntegrityChecked=true after policy-integrity passes"
                .to_owned(),
            None,
        )?);
    }
    if d.profile_name.as_deref() == Some("strict")
        && !d
            .fail_on
            .iter()
            .any(|value| value.eq_ignore_ascii_case("error"))
    {
        findings.push(finding(
            "CFG-1.1",
            file,
            "strict profiles must keep \"error\" in failOn".to_owned(),
            None,
        )?);
    }
    for (id, override_) in &d.rules {
        let Some(rule) = catalog.get(id) else {
            findings.push(finding(
                "ENF-1.3",
                file,
                format!("{id} is configured but not registered"),
                None,
            )?);
            continue;
        };
        if override_.enabled == Some(false) && rule.lock_level == "immutable" && !rule.can_disable {
            findings.push(finding(
                "CFG-1.2",
                file,
                format!("{id} is immutable and cannot be disabled"),
                None,
            )?);
        }
        if let Some(severity) = &override_.severity {
            if rule.lock_level == "immutable"
                && !rule.can_downgrade
                && severity_rank(severity) < severity_rank(&rule.severity)
            {
                findings.push(finding(
                    "CFG-1.3",
                    file,
                    format!(
                        "{id} is immutable and cannot be downgraded from {} to {severity}",
                        rule.severity
                    ),
                    None,
                )?);
            }
        }
        if override_.enabled == Some(false)
            && (rule.lock_level != "immutable" || rule.can_disable)
            && !override_.has_complete_disable_waiver
        {
            findings.push(finding("CFG-1.8", file, format!("{id} disable lacks waiverId, owner, issue, reason, scope, expires, and remediation"), None)?);
        }
    }
    if d.allow_unsafe_code && !has_waiver(d, "CFG-1.4") {
        findings.push(finding(
            "CFG-1.4",
            file,
            "allowUnsafeCode=true requires a narrow waiver".to_owned(),
            None,
        )?);
    }
    if d.public_reexport_policy.as_deref() == Some("allow")
        && d.profile_name.as_deref() == Some("strict")
        && !has_waiver(d, "CFG-1.5")
    {
        findings.push(finding(
            "CFG-1.5",
            file,
            "publicReexportPolicy=\"allow\" is forbidden in strict profiles".to_owned(),
            None,
        )?);
    }
    for (name, enabled) in [
        ("allowBuildRs", d.allow_build_rs),
        ("allowGitDependencies", d.allow_git_dependencies),
        ("allowPathDependencies", d.allow_path_dependencies),
    ] {
        if enabled && !has_waiver(d, "CFG-1.6") {
            findings.push(finding(
                "CFG-1.6",
                file,
                format!("{name}=true requires a narrow waiver"),
                None,
            )?);
        }
    }
    for field in &d.boundary_fields_without_owner_note {
        findings.push(finding(
            "CFG-1.7",
            file,
            format!("{field} changes require boundaryOwnerNote"),
            Some(field.clone()),
        )?);
    }
    for (name, values) in [
        ("sourceShapeOverrides", &d.source_shape_overrides),
        ("importBoundaryPolicies", &d.import_boundary_policies),
    ] {
        for value in values {
            if value.has_glob && !value.has_note {
                findings.push(finding(
                    "CFG-1.7",
                    file,
                    format!("{name} glob entries require note"),
                    None,
                )?);
            }
        }
    }
    Ok(findings)
}

fn waiver_findings(
    d: &ConfigParseDiagnostics,
    catalog: &BTreeMap<String, CatalogRule>,
    file: &str,
) -> Result<Vec<Finding>, String> {
    let mut findings = Vec::new();
    if d.max_active_waivers
        .is_some_and(|max| d.waivers.len() > max)
    {
        findings.push(finding(
            "WAIVER-1.7",
            file,
            format!(
                "active waiver count {} exceeds budget {}",
                d.waivers.len(),
                d.max_active_waivers.unwrap_or_default()
            ),
            None,
        )?);
    }
    let today = civil_today()?;
    for waiver in &d.waivers {
        let label = waiver
            .waiver_id
            .as_deref()
            .or(waiver.rule_id.as_deref())
            .unwrap_or("unnamed");
        let mut missing = Vec::new();
        for (name, value) in [
            ("ruleId", &waiver.rule_id),
            ("waiverId", &waiver.waiver_id),
            ("owner", &waiver.owner),
            ("issue", &waiver.issue),
            ("reason", &waiver.reason),
            ("expires", &waiver.expires),
            ("remediation", &waiver.remediation),
        ] {
            if value.is_none() {
                missing.push(name);
            }
        }
        if waiver.scope.as_ref().is_none_or(|scope| scope.is_empty()) {
            missing.push("scope");
        }
        if waiver.ci_allowed.is_none() {
            missing.push("ciAllowed");
        }
        if !missing.is_empty() {
            findings.push(finding(
                "WAIVER-1.1",
                file,
                format!("{label} waiver is missing: {}", missing.join(", ")),
                None,
            )?);
        }
        if waiver
            .scope
            .as_ref()
            .is_some_and(|scope| scope.iter().any(|scope| is_broad_scope(scope)))
        {
            findings.push(finding(
                "WAIVER-1.2",
                file,
                format!("{label} uses a broad waiver scope"),
                None,
            )?);
        }
        match waiver.expires.as_deref().and_then(parse_civil_date) {
            Some(expires) if expires >= today => {
                if days_between(today, expires) > d.max_waiver_days {
                    findings.push(finding(
                        "WAIVER-1.8",
                        file,
                        format!(
                            "{label} expiry exceeds max waiver window of {} days",
                            d.max_waiver_days
                        ),
                        None,
                    )?);
                }
            }
            _ => findings.push(finding(
                "WAIVER-1.3",
                file,
                format!("{label} is expired or has an invalid expiry"),
                None,
            )?),
        }
        if let Some(rule_id) = &waiver.rule_id {
            if let Some(rule) = catalog.get(rule_id) {
                if rule.lock_level == "immutable" && !rule.waivable {
                    findings.push(finding(
                        "WAIVER-1.4",
                        file,
                        format!("{label} attempts to waive immutable {rule_id}"),
                        None,
                    )?);
                }
            }
        }
        if std::env::var_os("CI").is_some() && waiver.ci_allowed != Some(true) {
            findings.push(finding(
                "WAIVER-1.5",
                file,
                format!("{label} is not CI-allowed"),
                None,
            )?);
        }
        if waiver.visible == Some(false) {
            findings.push(finding(
                "WAIVER-1.6",
                file,
                format!("{label} is hidden from output"),
                None,
            )?);
        }
        if waiver.owner.as_deref().is_some_and(|owner| {
            matches!(
                owner.trim().to_ascii_lowercase().as_str(),
                "ai" | "codex" | "agent" | "llm"
            )
        }) {
            findings.push(finding(
                "WAIVER-1.9",
                file,
                format!("{label} owner must be an accountable human or team"),
                None,
            )?);
        }
        if waiver.remediation.is_none() {
            findings.push(finding(
                "WAIVER-1.10",
                file,
                format!("{label} lacks a remediation plan"),
                None,
            )?);
        }
    }
    Ok(findings)
}

fn has_waiver(d: &ConfigParseDiagnostics, rule: &str) -> bool {
    d.waivers.iter().any(|waiver| {
        waiver
            .rule_id
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case(rule))
    })
}
fn severity_rank(value: &str) -> u8 {
    match value.to_ascii_lowercase().as_str() {
        "error" => 3,
        "warning" | "warn" => 2,
        "info" => 1,
        _ => 0,
    }
}
fn is_broad_scope(scope: &str) -> bool {
    let value = scope.replace('\\', "/").trim().to_owned();
    value.is_empty()
        || matches!(
            value.as_str(),
            "." | "/" | "**" | "**/*" | "src/**" | "crates/**" | "packages/**" | "apps/**"
        )
        || value.strip_prefix("**/*.").is_some_and(|extension| {
            extension
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        })
}

fn parse_civil_date(value: &str) -> Option<(i32, u32, u32)> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day))
}
fn civil_today() -> Result<(i32, u32, u32), String> {
    let seconds = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_secs(),
    )
    .map_err(|error| format!("system clock seconds do not fit i64: {error}"))?;
    let days = seconds / 86_400;
    civil_from_days(days)
}
fn civil_from_days(days: i64) -> Result<(i32, u32, u32), String> {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    Ok((
        i32::try_from(y + if m <= 2 { 1 } else { 0 })
            .map_err(|error| format!("civil year does not fit i32: {error}"))?,
        u32::try_from(m).map_err(|error| format!("civil month does not fit u32: {error}"))?,
        u32::try_from(d).map_err(|error| format!("civil day does not fit u32: {error}"))?,
    ))
}
fn days_between(start: (i32, u32, u32), end: (i32, u32, u32)) -> usize {
    let days = |(year, month, day): (i32, u32, u32)| {
        let year = i64::from(year) - if month <= 2 { 1 } else { 0 };
        let era = if year >= 0 { year } else { year - 399 } / 400;
        let yoe = year - era * 400;
        let mp = i64::from(month) + if month > 2 { -3 } else { 9 };
        let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
        era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
    };
    usize::try_from(days(end) - days(start)).unwrap_or_default()
}

fn report(scope: ScanScope, findings: Vec<Finding>) -> Result<Report, String> {
    let violations = findings
        .iter()
        .cloned()
        .filter_map(|value| Violation::try_from(value).ok())
        .collect::<Vec<_>>();
    Ok(Report {
        ok: if violations.is_empty() {
            ReportOutcome::Clean
        } else {
            ReportOutcome::Violations
        },
        scope,
        violations,
        warnings: Vec::new(),
        waived: Vec::new(),
        findings,
    })
}
fn finding(
    rule: &str,
    file: &str,
    detail: String,
    snippet: Option<String>,
) -> Result<Finding, String> {
    Ok(Finding {
        rule_id: rule.parse::<RuleId>().map_err(|e| e.to_string())?,
        severity: Severity::Error,
        title: FindingTitle::new("configuration governance violation".to_owned())
            .map_err(|e| e.to_string())?,
        detail: FindingDetail::new(detail).map_err(|e| e.to_string())?,
        snippet: snippet.and_then(|value| FindingSnippet::new(value).ok()),
        file: file.parse::<RelPath>().map_err(|e| e.to_string())?,
        line: FindingLine::known(SourceLine::try_new(NonZeroU32::new(1).ok_or("line")?)),
    })
}

#[cfg(test)]
mod tests {
    use super::{check_config_lockdown, check_waiver_policy};
    use enforcer_domain::findings::ScanScope;
    fn seed(root: &std::path::Path, config: &str) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(root.join("rules"))?;
        std::fs::write(
            root.join("rules/rules.json"),
            r#"{"rules":[{"id":"RR-4.1","severity":"error","lockLevel":"immutable","canDisable":false,"canDowngrade":false,"waivable":false},{"id":"RR-9.1","severity":"warning","lockLevel":"advisory","canDisable":true,"canDowngrade":true,"waivable":true}]}"#,
        )?;
        std::fs::write(root.join("ocentra-enforcer.config.json"), config)?;
        Ok(())
    }
    #[test]
    fn lockdown_reports_the_full_governance_surface() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        seed(
            temp.path(),
            r#"{"schemaVersion":2,"profileName":"strict","failOn":[],"configChangeRequiresSelfCheck":true,"rules":{"RR-4.1":{"enabled":false,"severity":"warning"},"RR-9.1":{"enabled":false},"NOPE-1.1":{"enabled":false}},"allowUnsafeCode":true,"allowBuildRs":true,"rawTypeBoundaryGlobs":["src/**"],"sourceShapeOverrides":[{"glob":"src/**"}]}"#,
        )?;
        let report = check_config_lockdown(
            &temp.path().join("ocentra-enforcer.config.json"),
            temp.path(),
            ScanScope::Workspace,
        )?;
        for id in [
            "CFG-1.1", "CFG-1.2", "CFG-1.3", "CFG-1.4", "CFG-1.6", "CFG-1.7", "CFG-1.8",
            "CFG-1.12", "ENF-1.3",
        ] {
            assert!(
                report
                    .findings
                    .iter()
                    .any(|value| value.rule_id.as_str() == id),
                "missing {id}"
            );
        }
        Ok(())
    }
    #[test]
    fn waiver_policy_reports_required_governance() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        seed(
            temp.path(),
            r#"{"schemaVersion":2,"profileName":"strict","waivers":[{"ruleId":"RR-4.1","waiverId":"w","owner":"codex","scope":["**/*"],"expires":"2000-01-01","visible":false}],"maxActiveWaivers":0}"#,
        )?;
        let report = check_waiver_policy(
            &temp.path().join("ocentra-enforcer.config.json"),
            temp.path(),
            ScanScope::Workspace,
        )?;
        for id in [
            "WAIVER-1.1",
            "WAIVER-1.2",
            "WAIVER-1.3",
            "WAIVER-1.4",
            "WAIVER-1.6",
            "WAIVER-1.7",
            "WAIVER-1.9",
            "WAIVER-1.10",
        ] {
            assert!(
                report
                    .findings
                    .iter()
                    .any(|value| value.rule_id.as_str() == id),
                "missing {id}"
            );
        }
        Ok(())
    }

    #[test]
    fn governance_checks_fail_closed_when_config_cannot_be_decoded(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        seed(temp.path(), "{not-json")?;
        let config = temp.path().join("ocentra-enforcer.config.json");

        for result in [
            check_config_lockdown(&config, temp.path(), ScanScope::Workspace),
            check_waiver_policy(&config, temp.path(), ScanScope::Workspace),
        ] {
            let error = result.expect_err("malformed configuration must fail closed");
            assert!(error.contains("cannot inspect project configuration"));
        }
        Ok(())
    }
}
