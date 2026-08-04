//! BOUNDARY-INVARIANT: raw daemon inputs are validated here before the process-local service is started.
//! Negative invalid-input coverage rejects non-loopback hosts and unauthorized health requests.
//! Process-local authenticated coordination HTTP service used by MCP `ensure`.
//! It never persists the supplied token and only binds loopback addresses.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Mutex, OnceLock};

use crate::error::{CoordinationError, Result};
use enforcer_domain::coordination_types::CoordinationLedgerRoot;
use enforcer_domain::coordination_types::CoordinationRejection;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Address and reuse status for the process-local coordination service.
pub struct EnsureStatus {
    pub host: String,
    pub port: u16,
    pub reused: bool,
}

impl serde::Serialize for EnsureStatus {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("EnsureStatus", 3)?;
        state.serialize_field("host", &self.host)?;
        state.serialize_field("port", &self.port)?;
        state.serialize_field("reused", &self.reused)?;
        state.end()
    }
}

static SERVICES: OnceLock<Mutex<BTreeMap<String, u16>>> = OnceLock::new();

/// Ensure an authenticated loopback coordination service is listening.
pub fn ensure(
    root: &CoordinationLedgerRoot,
    host: &str,
    port: u16,
    token: Option<&str>,
) -> Result<EnsureStatus> {
    if host != "127.0.0.1" && host != "localhost" {
        return Err(rejected("coordination ensure only permits loopback hosts"));
    }
    let key = format!("{host}:{port}");
    let services = SERVICES.get_or_init(|| Mutex::new(BTreeMap::new()));
    if services
        .lock()
        .map_err(|_poisoned| rejected("coordination daemon registry lock poisoned"))?
        .contains_key(&key)
    {
        return Ok(EnsureStatus {
            host: host.to_owned(),
            port,
            reused: true,
        });
    }
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(CoordinationError::Io)?;
    let actual_port = listener.local_addr().map_err(CoordinationError::Io)?.port();
    let expected_token = token.map(str::to_owned);
    let ledger_root = root.as_path().to_path_buf();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut stream = stream;
            let mut request = [0_u8; 4096];
            let Ok(read) = stream.read(&mut request) else {
                continue;
            };
            let Some(bytes) = request.get(..read) else {
                continue;
            };
            let request = String::from_utf8_lossy(bytes);
            let authorized = expected_token
                .as_ref()
                .is_none_or(|value| authorization_bearer(&request) == Some(value.as_str()));
            let response = if !authorized {
                http_response("401 Unauthorized", "application/json", "")
            } else {
                serve_request(&ledger_root, &request)
            };
            let _ = stream.write_all(&response);
        }
    });
    services
        .lock()
        .map_err(|_poisoned| rejected("coordination daemon registry lock poisoned"))?
        .insert(format!("{host}:{actual_port}"), actual_port);
    Ok(EnsureStatus {
        host: host.to_owned(),
        port: actual_port,
        reused: false,
    })
}

fn authorization_bearer(request: &str) -> Option<&str> {
    request.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if !name.trim().eq_ignore_ascii_case("authorization") {
            return None;
        }
        value.trim().strip_prefix("Bearer ")
    })
}

fn serve_request(root: &std::path::Path, request: &str) -> Vec<u8> {
    let target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1));
    match target {
        Some("/health") => http_response("200 OK", "application/json", "{\"ok\":true}"),
        Some("/manifest") => match crate::sync::stream::list_stream_files(root) {
            Ok(streams) => {
                let body = serde_json::json!({
                    "streams": streams.iter().map(|stream| stream.as_str()).collect::<Vec<_>>()
                })
                .to_string();
                http_response("200 OK", "application/json", &body)
            }
            Err(_) => http_response("500 Internal Server Error", "application/json", ""),
        },
        Some(target) if target.starts_with("/streams/") => {
            let requested = target.trim_start_matches("/streams/").replace("%25", "%");
            match crate::sync::stream::list_stream_files(root).and_then(|streams| {
                streams
                    .into_iter()
                    .find(|stream| stream.as_str() == requested)
                    .ok_or_else(|| rejected("coordination stream was not found"))
            }) {
                Ok(stream_name) => {
                    match std::fs::read_to_string(root.join("streams").join(stream_name.as_str())) {
                        Ok(body) => http_response("200 OK", "application/x-ndjson", &body),
                        Err(_) => {
                            http_response("500 Internal Server Error", "application/json", "")
                        }
                    }
                }
                Err(_) => http_response("404 Not Found", "application/json", ""),
            }
        }
        _ => http_response("404 Not Found", "application/json", ""),
    }
}

fn http_response(status: &str, content_type: &str, body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn rejected(message: &'static str) -> CoordinationError {
    match CoordinationRejection::from_static(message) {
        Ok(reason) => CoordinationError::rejected(reason),
        Err(error) => CoordinationError::Decode(error),
    }
}

#[cfg(test)]
mod tests {
    use super::ensure;
    use crate::error::CoordinationError;
    use enforcer_domain::coordination_types::CoordinationLedgerRoot;
    use std::io::{Read, Write};
    use std::net::TcpStream;

    fn request(
        port: u16,
        target: &str,
        authorization: Option<(&str, &str)>,
    ) -> Result<String, std::io::Error> {
        let mut stream = TcpStream::connect(("127.0.0.1", port))?;
        let auth = match authorization {
            Some((name, value)) => format!("{name}: Bearer {value}\r\n"),
            None => String::new(),
        };
        stream.write_all(
            format!("GET {target} HTTP/1.1\r\nHost: localhost\r\n{auth}\r\n").as_bytes(),
        )?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        Ok(response)
    }

    #[test]
    fn ensure_is_idempotent_and_authenticates_health() -> crate::error::Result<()> {
        let root_dir = tempfile::tempdir()?;
        let root = CoordinationLedgerRoot::parse(root_dir.path())?;
        let initial = ensure(&root, "127.0.0.1", 0, Some("token"))?;
        let repeated = ensure(&root, "127.0.0.1", initial.port, Some("different-token"))?;
        assert!(repeated.reused);
        assert_eq!(
            serde_json::to_value(&initial)?,
            serde_json::json!({
                "host": "127.0.0.1",
                "port": initial.port,
                "reused": false,
            })
        );
        assert!(request(initial.port, "/health", None)?.starts_with("HTTP/1.1 401"));
        assert!(
            request(initial.port, "/health", Some(("authorization", "token")))?
                .starts_with("HTTP/1.1 200")
        );
        Ok(())
    }

    #[test]
    fn ensured_service_serves_manifest_and_stream_routes() -> crate::error::Result<()> {
        let root_dir = tempfile::tempdir()?;
        std::fs::create_dir_all(root_dir.path().join("streams"))?;
        std::fs::write(
            root_dir.path().join("streams/node_lane.ndjson"),
            "{\"seq\":1}\n",
        )?;
        let root = CoordinationLedgerRoot::parse(root_dir.path())?;
        let service = ensure(&root, "127.0.0.1", 0, None)?;
        let manifest = request(service.port, "/manifest", None)?;
        assert!(manifest.starts_with("HTTP/1.1 200"));
        assert!(manifest.contains("node_lane.ndjson"));
        let stream = request(service.port, "/streams/node_lane.ndjson", None)?;
        assert!(stream.starts_with("HTTP/1.1 200"));
        assert!(stream.ends_with("{\"seq\":1}\n"));
        Ok(())
    }

    #[test]
    fn ensure_rejects_non_loopback_bind() -> Result<(), String> {
        let root_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let root =
            CoordinationLedgerRoot::parse(root_dir.path()).map_err(|error| error.to_string())?;
        let error = ensure(&root, "0.0.0.0", 8787, None)
            .err()
            .ok_or("non-loopback daemon binds must be rejected")?;
        match error {
            CoordinationError::Rejected(reason) => assert_eq!(
                reason.as_str(),
                "coordination ensure only permits loopback hosts"
            ),
            other => return Err(format!("expected a typed rejection, got {other:?}")),
        }
        Ok(())
    }
}
