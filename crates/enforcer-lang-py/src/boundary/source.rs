//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Python source position conversions.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::paths::RelPath;

/// Python source layers recognized by the layered-architecture validators.
#[derive(Debug, Clone, Copy)]
pub(crate) enum PythonLayer {
    Routers,
    Services,
    Workflows,
    Domain,
}

impl PythonLayer {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Routers => "routers",
            Self::Services => "services",
            Self::Workflows => "workflows",
            Self::Domain => "domain",
        }
    }
}

/// Static source markers owned by a validator definition.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PythonMarkers(&'static [&'static str]);

impl PythonMarkers {
    pub(crate) const fn new(markers: &'static [&'static str]) -> Self {
        Self(markers)
    }

    pub(crate) fn any_in(self, source: ValidationSource<'_>) -> bool {
        self.0.iter().any(|marker| source.as_str().contains(marker))
    }

    pub(crate) fn iter(self) -> impl Iterator<Item = &'static str> {
        self.0.iter().copied()
    }
}

/// Supported shapes for structured Python tool diagnostics.
#[derive(Debug, Clone, Copy)]
pub(crate) enum DiagnosticsArray {
    Root,
    GeneralDiagnostics,
}

pub(crate) fn diagnostics_count(source: ValidationSource<'_>, array: DiagnosticsArray) -> usize {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(source.as_str()) else {
        return 0;
    };
    let diagnostics = match array {
        DiagnosticsArray::Root => parsed.as_array(),
        DiagnosticsArray::GeneralDiagnostics => parsed
            .get("generalDiagnostics")
            .and_then(serde_json::Value::as_array),
    };
    diagnostics.map_or(0, Vec::len)
}

pub(crate) fn one_based_line(index: usize) -> Result<u32, DecodeError> {
    let line = index
        .checked_add(1)
        .ok_or_else(|| DecodeError::new("python.source.line", "line index overflow"))?;
    u32::try_from(line)
        .map_err(|_source| DecodeError::new("python.source.line", "line exceeds u32 range"))
}

pub(crate) fn in_layer(path: &RelPath, layer: PythonLayer) -> bool {
    let path = path.as_str();
    let segment = layer.as_str();
    path.contains(&format!("/{segment}/")) || path.starts_with(&format!("{segment}/"))
}

pub(crate) fn code_contains(source: &str, marker: &str) -> bool {
    source.lines().any(|line| code_part(line).contains(marker))
}

pub(crate) fn first_code_line_with(source: &str, marker: &str) -> Option<u32> {
    source
        .lines()
        .enumerate()
        .find(|(_, line)| code_part(line).contains(marker))
        .and_then(|(index, _)| one_based_line(index).ok())
}

pub(crate) fn code_part(line: &str) -> &str {
    match line.find('#') {
        Some(index) => line.get(..index).unwrap_or(line),
        None => line,
    }
}

pub(crate) fn imports_from_models_package(source: &str) -> Option<u32> {
    source.lines().enumerate().find_map(|(index, line)| {
        let code = code_part(line).trim_start();
        let module = if let Some(rest) = code.strip_prefix("from ") {
            rest.split(" import").next().unwrap_or("")
        } else if let Some(rest) = code.strip_prefix("import ") {
            rest.split([' ', ',']).next().unwrap_or("")
        } else {
            return None;
        };
        module
            .split('.')
            .any(|segment| segment == "models")
            .then(|| one_based_line(index).ok())
            .flatten()
    })
}
