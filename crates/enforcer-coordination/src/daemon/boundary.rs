//! BOUNDARY-INVARIANT: raw daemon inputs are validated here before the process-local service is started.
//! Negative invalid-input coverage rejects non-loopback hosts and unauthorized health requests.
//! Process-local authenticated coordination HTTP service used by MCP `ensure`.
//! It never persists the supplied token and only binds loopback addresses.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Mutex, OnceLock};

use crate::error::{CoordinationError, Result};
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
pub fn ensure(host: &str, port: u16, token: Option<&str>) -> Result<EnsureStatus> {
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
                .is_none_or(|value| request.contains(&format!("Authorization: Bearer {value}")));
            let response = if authorized && request.starts_with("GET /health ") {
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\n\r\n{\"ok\":true}"
            } else {
                "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n"
            };
            let _ = stream.write_all(response.as_bytes());
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
    use std::io::{Read, Write};
    use std::net::TcpStream;

    fn request(port: u16, authorization: Option<&str>) -> Result<String, std::io::Error> {
        let mut stream = TcpStream::connect(("127.0.0.1", port))?;
        let auth = match authorization {
            Some(value) => format!("Authorization: Bearer {value}\r\n"),
            None => String::new(),
        };
        stream.write_all(
            format!("GET /health HTTP/1.1\r\nHost: localhost\r\n{auth}\r\n").as_bytes(),
        )?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        Ok(response)
    }

    #[test]
    fn ensure_is_idempotent_and_authenticates_health() -> crate::error::Result<()> {
        let initial = ensure("127.0.0.1", 0, Some("token"))?;
        let repeated = ensure("127.0.0.1", initial.port, Some("different-token"))?;
        assert!(repeated.reused);
        assert_eq!(
            serde_json::to_value(&initial)?,
            serde_json::json!({
                "host": "127.0.0.1",
                "port": initial.port,
                "reused": false,
            })
        );
        assert!(request(initial.port, None)?.starts_with("HTTP/1.1 401"));
        assert!(request(initial.port, Some("token"))?.starts_with("HTTP/1.1 200"));
        Ok(())
    }

    #[test]
    fn ensure_rejects_non_loopback_bind() -> Result<(), String> {
        let error = ensure("0.0.0.0", 8787, None)
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
