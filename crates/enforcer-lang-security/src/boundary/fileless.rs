//! Fileless-investigation telemetry decoded from an external JSON snapshot.
//!
//! BOUNDARY-INVARIANT: malformed or unknown telemetry snapshots are rejected
//! before prerequisite state reaches the fileless-investigation validator.
//! Malformed JSON has negative coverage in this module's tests.

use enforcer_domain::boundary::decode_error::DecodeError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc = "A typed prerequisite required for trustworthy fileless investigation."]
pub(crate) enum TelemetryRequirement {
    BaselineKind,
    SysmonProcessCreation,
    SysmonWmiEvent19,
    SysmonWmiEvent20,
    SysmonWmiEvent21,
    SysmonRegistryChanges,
    SysmonNetworkConnections,
    PowerShellScriptBlockLogging,
    PowerShellModuleLogging,
    PowerShellEvent4104,
    WindowsEventLogRetention,
    Volatility3,
    ProcessMonitor,
    Autoruns,
}

impl TelemetryRequirement {
    /// Return the diagnostic label for this prerequisite.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::BaselineKind => "kind=fileless-telemetry-baseline",
            Self::SysmonProcessCreation => "Sysmon process creation logging",
            Self::SysmonWmiEvent19 => "Sysmon WMI Event ID 19",
            Self::SysmonWmiEvent20 => "Sysmon WMI Event ID 20",
            Self::SysmonWmiEvent21 => "Sysmon WMI Event ID 21",
            Self::SysmonRegistryChanges => "Sysmon registry-change logging",
            Self::SysmonNetworkConnections => "Sysmon network-connection logging",
            Self::PowerShellScriptBlockLogging => "PowerShell Script Block Logging",
            Self::PowerShellModuleLogging => "PowerShell Module Logging",
            Self::PowerShellEvent4104 => "PowerShell Script Block Event ID 4104",
            Self::WindowsEventLogRetention => "positive Windows Event Log retention",
            Self::Volatility3 => "Volatility 3 availability",
            Self::ProcessMonitor => "Process Monitor availability",
            Self::Autoruns => "Autoruns availability",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
enum TelemetryKind {
    #[serde(rename = "fileless-telemetry-baseline")]
    FilelessTelemetryBaseline,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SysmonSnapshot {
    process_creation: bool,
    wmi_events: Vec<u64>,
    registry_changes: bool,
    network_connections: bool,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PowerShellSnapshot {
    script_block_logging: bool,
    module_logging: bool,
    // DEFAULT-JUSTIFICATION: an absent event list proves no retained Script Block event type.
    #[serde(default)]
    event_ids: Vec<u64>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindowsEventLogSnapshot {
    retention_days: u64,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvestigationToolsSnapshot {
    volatility3: bool,
    process_monitor: bool,
    autoruns: bool,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[doc = "Typed prerequisite state decoded from the external telemetry snapshot."]
pub(crate) struct FilelessTelemetryBaseline {
    kind: Option<TelemetryKind>,
    // DEFAULT-JUSTIFICATION: an absent Sysmon section proves no prerequisite telemetry.
    #[serde(default)]
    sysmon: SysmonSnapshot,
    // DEFAULT-JUSTIFICATION: an absent PowerShell section proves no prerequisite logging.
    #[serde(default)]
    powershell: PowerShellSnapshot,
    // DEFAULT-JUSTIFICATION: absent retention policy means no positive retention is proven.
    #[serde(default)]
    windows_event_log: WindowsEventLogSnapshot,
    // DEFAULT-JUSTIFICATION: an absent tools section proves no investigation tool availability.
    #[serde(default)]
    tools: InvestigationToolsSnapshot,
}

impl FilelessTelemetryBaseline {
    /// Return whether the snapshot explicitly declares the canonical baseline kind.
    pub(crate) fn declares_baseline_kind(&self) -> bool {
        self.kind == Some(TelemetryKind::FilelessTelemetryBaseline)
    }

    /// Return every prerequisite that the snapshot fails to prove enabled.
    pub(crate) fn missing_requirements(&self) -> Vec<TelemetryRequirement> {
        let checks = [
            (
                TelemetryRequirement::BaselineKind,
                self.declares_baseline_kind(),
            ),
            (
                TelemetryRequirement::SysmonProcessCreation,
                self.sysmon.process_creation,
            ),
            (
                TelemetryRequirement::SysmonWmiEvent19,
                self.sysmon.wmi_events.contains(&19),
            ),
            (
                TelemetryRequirement::SysmonWmiEvent20,
                self.sysmon.wmi_events.contains(&20),
            ),
            (
                TelemetryRequirement::SysmonWmiEvent21,
                self.sysmon.wmi_events.contains(&21),
            ),
            (
                TelemetryRequirement::SysmonRegistryChanges,
                self.sysmon.registry_changes,
            ),
            (
                TelemetryRequirement::SysmonNetworkConnections,
                self.sysmon.network_connections,
            ),
            (
                TelemetryRequirement::PowerShellScriptBlockLogging,
                self.powershell.script_block_logging,
            ),
            (
                TelemetryRequirement::PowerShellModuleLogging,
                self.powershell.module_logging,
            ),
            (
                TelemetryRequirement::PowerShellEvent4104,
                self.powershell.event_ids.contains(&4104),
            ),
            (
                TelemetryRequirement::WindowsEventLogRetention,
                self.windows_event_log.retention_days > 0,
            ),
            (TelemetryRequirement::Volatility3, self.tools.volatility3),
            (
                TelemetryRequirement::ProcessMonitor,
                self.tools.process_monitor,
            ),
            (TelemetryRequirement::Autoruns, self.tools.autoruns),
        ];
        checks
            .into_iter()
            .filter_map(|(requirement, present)| (!present).then_some(requirement))
            .collect()
    }
}

/// Decode an untrusted JSON telemetry snapshot into typed prerequisite state.
pub(crate) fn decode(source: &str) -> Result<FilelessTelemetryBaseline, DecodeError> {
    serde_json::from_str(source).map_err(|error| {
        DecodeError::new("filelessTelemetryBaseline", error.to_string())
            .with_input_hint("expected the documented fileless telemetry JSON snapshot")
    })
}

/// Return whether one registry evidence line carries a large encoded payload.
pub(crate) fn contains_large_registry_payload(line: &str) -> bool {
    if line.len() <= 500 {
        return false;
    }
    let normalized = line.to_ascii_lowercase().replace("\\\\", "\\");
    let registry_context = normalized.contains("hkcu\\software")
        || normalized.contains("hkey_current_user\\software")
        || normalized.contains("hklm\\software")
        || normalized.contains("hkey_local_machine\\software")
        || normalized.contains("hkcu\\environment")
        || normalized.contains("hkey_current_user\\environment");
    if !registry_context {
        return false;
    }
    let has_powershell_marker = normalized.contains("powershell")
        || normalized.contains("invoke-")
        || normalized.contains("-encodedcommand")
        || normalized.contains("-enc ")
        || normalized
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|token| token == "iex");
    let has_large_base64_token = line
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || matches!(character, '+' | '/' | '='))
        })
        .any(|token| token.len() > 500);
    has_powershell_marker || has_large_base64_token
}

#[cfg(test)]
mod tests {
    #[test]
    fn malformed_fileless_telemetry_snapshot_is_rejected() {
        assert!(super::decode(r#"{"kind":"fileless-telemetry-baseline""#).is_err());
    }

    #[test]
    fn large_encoded_value_requires_registry_context() {
        let payload = "A".repeat(520);
        assert!(super::contains_large_registry_payload(&format!(
            r"HKCU\Software\Updater\Payload = {payload}"
        )));
        assert!(!super::contains_large_registry_payload(&format!(
            "releaseNotes = {payload}"
        )));
    }
}
