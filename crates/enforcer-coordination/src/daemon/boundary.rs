//! Process-local authenticated coordination HTTP service used by MCP `ensure`.
//! It never persists the supplied token and only binds loopback addresses.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct EnsureStatusDto {
    pub host: String,
    pub port: u16,
    pub reused: bool,
}

static SERVICES: OnceLock<Mutex<BTreeMap<String, u16>>> = OnceLock::new();

pub fn ensure(host: &str, port: u16, token: Option<&str>) -> Result<EnsureStatusDto, String> {
    if host != "127.0.0.1" && host != "localhost" {
        return Err("coordination ensure only permits loopback hosts".to_owned());
    }
    let key = format!("{host}:{port}");
    let services = SERVICES.get_or_init(|| Mutex::new(BTreeMap::new()));
    if services
        .lock()
        .map_err(|_| "coordination daemon registry lock poisoned")?
        .contains_key(&key)
    {
        return Ok(EnsureStatusDto {
            host: host.to_owned(),
            port,
            reused: true,
        });
    }
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|error| format!("coordination ensure bind failed: {error}"))?;
    let actual_port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
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
        .map_err(|_| "coordination daemon registry lock poisoned")?
        .insert(format!("{host}:{actual_port}"), actual_port);
    Ok(EnsureStatusDto {
        host: host.to_owned(),
        port: actual_port,
        reused: false,
    })
}

#[cfg(test)]
mod tests {
    use super::ensure;
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
    fn ensure_is_idempotent_and_authenticates_health() -> Result<(), String> {
        let initial = ensure("127.0.0.1", 0, Some("token"))?;
        let repeated = ensure("127.0.0.1", initial.port, Some("different-token"))?;
        assert!(repeated.reused);
        assert!(request(initial.port, None)
            .map_err(|error| error.to_string())?
            .starts_with("HTTP/1.1 401"));
        assert!(request(initial.port, Some("token"))
            .map_err(|error| error.to_string())?
            .starts_with("HTTP/1.1 200"));
        Ok(())
    }

    #[test]
    fn ensure_rejects_non_loopback_bind() {
        assert!(ensure("0.0.0.0", 8787, None).is_err());
    }
}
