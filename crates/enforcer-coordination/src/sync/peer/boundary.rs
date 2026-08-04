//! Serialized peer-registry and HTTP-manifest boundary shapes.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};

use crate::error::CoordinationError;
use crate::error::Result;
use crate::sync::stream::{list_stream_files, read_lines};
use enforcer_domain::coordination_types::{
    CoordinationLedgerRoot, CoordinationPeerName, CoordinationPeerTokenEnv, CoordinationPeerUrl,
    CoordinationRejection,
};

#[derive(Clone)]
/// Validated peer-registry record with a redacted token environment name.
pub struct PeerRecord {
    pub name: CoordinationPeerName,
    pub url: CoordinationPeerUrl,
    pub token_env: Option<CoordinationPeerTokenEnv>,
}
impl std::fmt::Debug for PeerRecord {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PeerRecord")
            .field("name", &self.name)
            .field("url", &self.url)
            .field("token_env", &"[redacted]")
            .finish()
    }
}
#[derive(Debug, Clone, Default)]
/// Persisted collection of configured coordination peers.
pub struct PeerRegistry {
    pub peers: Vec<PeerRecord>,
}
#[derive(Debug, Clone)]
/// Bounded result of importing append-only peer stream suffixes.
pub struct SyncResult {
    pub imported: usize,
    pub transferred_lines: usize,
    pub conflicts: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
/// Serialized peer-registry boundary representation.
pub struct PeerRegistryDto {
    pub peers: Vec<PeerRegistryEntryDto>,
}
impl std::fmt::Debug for PeerRegistryDto {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PeerRegistryDto")
            .field("peers", &self.peers)
            .finish()
    }
}
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
/// Serialized entry in a peer registry.
pub struct PeerRegistryEntryDto {
    pub name: String,
    pub url: String,
    pub token_env: Option<String>,
    pub mode: Option<String>,
}
impl std::fmt::Debug for PeerRegistryEntryDto {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PeerRegistryEntryDto")
            .field("name", &self.name)
            .field("url", &self.url)
            .field("token_env", &"[redacted]")
            .field("mode", &self.mode)
            .finish()
    }
}
impl TryFrom<PeerRegistryDto> for PeerRegistry {
    type Error = crate::error::CoordinationError;
    fn try_from(value: PeerRegistryDto) -> Result<Self> {
        Ok(Self {
            peers: value
                .peers
                .into_iter()
                .map(|entry| {
                    Ok(PeerRecord {
                        name: CoordinationPeerName::parse(&entry.name)?,
                        url: CoordinationPeerUrl::parse(&entry.url)?,
                        token_env: entry
                            .token_env
                            .map(CoordinationPeerTokenEnv::parse)
                            .transpose()?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        })
    }
}
impl From<&PeerRegistry> for PeerRegistryDto {
    fn from(value: &PeerRegistry) -> Self {
        Self {
            peers: value
                .peers
                .iter()
                .map(|peer| PeerRegistryEntryDto {
                    name: peer.name.as_str().to_owned(),
                    url: peer.url.as_str().to_owned(),
                    token_env: peer
                        .token_env
                        .as_ref()
                        .map(|token| token.as_str().to_owned()),
                    mode: Some("pull".to_owned()),
                })
                .collect(),
        }
    }
}
/// Decode and validate safe stream names from a peer manifest.
pub fn decode_manifest(raw: &str) -> Result<Vec<String>> {
    #[derive(serde::Deserialize)]
    struct Manifest {
        streams: Vec<Stream>,
    }
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum Stream {
        Name(String),
        Entry { stream: String },
    }
    let parsed: Manifest = serde_json::from_str(raw)?;
    parsed
        .streams
        .into_iter()
        .map(|value| {
            let name = match value {
                Stream::Name(name) => name,
                Stream::Entry { stream } => stream,
            };
            if name.ends_with(".ndjson")
                && !name.contains(".conflict.")
                && !name.contains('/')
                && !name.contains('\\')
            {
                Ok(name)
            } else {
                Err(rejected("peer manifest contains an unsafe stream name"))
            }
        })
        .collect()
}

/// Load the peer registry, treating an absent file as an empty registry.
pub fn load_registry(root: &CoordinationLedgerRoot) -> Result<PeerRegistry> {
    let raw = match fs::read_to_string(registry_path(root.as_path())) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PeerRegistry::default())
        }
        Err(error) => return Err(error.into()),
    };
    serde_json::from_str::<PeerRegistryDto>(&raw)?.try_into()
}
/// Insert or replace a peer and persist the deterministically ordered registry.
pub fn add_peer(root: &CoordinationLedgerRoot, peer: PeerRecord) -> Result<PeerRegistry> {
    let mut registry = load_registry(root)?;
    registry.peers.retain(|existing| existing.name != peer.name);
    registry.peers.push(peer);
    registry
        .peers
        .sort_by(|left, right| left.name.as_str().cmp(right.name.as_str()));
    save_registry(root, &registry)?;
    Ok(registry)
}
/// Remove a named peer and persist the resulting registry.
pub fn remove_peer(
    root: &CoordinationLedgerRoot,
    name: &CoordinationPeerName,
) -> Result<PeerRegistry> {
    let mut registry = load_registry(root)?;
    registry.peers.retain(|peer| &peer.name != name);
    save_registry(root, &registry)?;
    Ok(registry)
}
/// Resolve one configured peer by its validated name.
pub fn resolve_peer(
    root: &CoordinationLedgerRoot,
    name: &CoordinationPeerName,
) -> Result<PeerRecord> {
    load_registry(root)?
        .peers
        .into_iter()
        .find(|peer| &peer.name == name)
        .ok_or_else(|| rejected("unknown peer alias"))
}
/// Resolve an optional peer token without persisting or logging its value.
pub fn token_from_env(token_env: Option<&CoordinationPeerTokenEnv>) -> Result<Option<String>> {
    token_env
        .map(|name| {
            std::env::var(name.as_str()).map_err(|_missing| {
                rejected("configured peer token environment variable is not set")
            })
        })
        .transpose()
}
/// Import compatible suffixes from a local peer ledger.
pub fn sync_local(root: &CoordinationLedgerRoot, peer_root: &Path) -> Result<SyncResult> {
    sync_lines(
        root,
        list_stream_files(peer_root)?
            .into_iter()
            .map(|stream| {
                let name = stream.as_str().to_owned();
                Ok((
                    name,
                    read_lines(&peer_root.join("streams").join(stream.as_str()))?,
                ))
            })
            .collect::<Result<Vec<_>>>()?,
    )
}
/// Import compatible suffixes from an authenticated HTTP peer.
pub fn sync_http(
    root: &CoordinationLedgerRoot,
    url: &CoordinationPeerUrl,
    token: Option<&str>,
) -> Result<SyncResult> {
    let streams = decode_manifest(&http_get(url, "/manifest", token)?)?;
    let remote = streams
        .into_iter()
        .map(|name| {
            let escaped = name.replace('%', "%25").replace('/', "%2F");
            Ok((
                name,
                split_lines(&http_get(url, &format!("/streams/{escaped}"), token)?),
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    sync_lines(root, remote)
}
fn sync_lines(
    root: &CoordinationLedgerRoot,
    remote: Vec<(String, Vec<String>)>,
) -> Result<SyncResult> {
    let dir = root.as_path().join("streams");
    fs::create_dir_all(&dir)?;
    let mut result = SyncResult {
        imported: 0,
        transferred_lines: 0,
        conflicts: Vec::new(),
    };
    for (name, peer_lines) in remote {
        let local = read_lines(&dir.join(&name))?;
        if local.len() > peer_lines.len()
            || !local
                .iter()
                .zip(&peer_lines)
                .all(|(left, right)| left == right)
        {
            result
                .conflicts
                .push(write_conflict(&dir, &name, &peer_lines)?);
            continue;
        }
        let suffix = peer_lines
            .get(local.len()..)
            .ok_or_else(|| rejected("peer suffix range invalid"))?;
        append_suffix(&dir.join(&name), suffix)?;
        result.imported += suffix.len();
        result.transferred_lines += suffix.len();
    }
    Ok(result)
}
fn registry_path(root: &Path) -> PathBuf {
    root.join("peers.json")
}
fn save_registry(root: &CoordinationLedgerRoot, registry: &PeerRegistry) -> Result<()> {
    fs::create_dir_all(root.as_path())?;
    fs::write(
        registry_path(root.as_path()),
        serde_json::to_vec_pretty(&PeerRegistryDto::from(registry))?,
    )?;
    Ok(())
}
fn append_suffix(path: &Path, suffix: &[String]) -> Result<()> {
    if suffix.is_empty() {
        return Ok(());
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for line in suffix {
        writeln!(file, "{line}")?
    }
    file.sync_all()?;
    Ok(())
}
fn split_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}
fn write_conflict(dir: &Path, name: &str, lines: &[String]) -> Result<String> {
    let conflict = format!(
        "{name}.conflict.{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |value| value.as_millis())
    );
    fs::write(dir.join(&conflict), lines.join("\n") + "\n")?;
    Ok(conflict)
}
fn rejected(message: &'static str) -> CoordinationError {
    match CoordinationRejection::from_static(message) {
        Ok(reason) => CoordinationError::rejected(reason),
        Err(error) => CoordinationError::Decode(error),
    }
}
fn http_get(base: &CoordinationPeerUrl, path: &str, token: Option<&str>) -> Result<String> {
    let endpoint = base
        .as_str()
        .strip_prefix("http://")
        .ok_or_else(|| rejected("native peer transport accepts http endpoints only"))?;
    let (authority, prefix) = endpoint.split_once('/').unwrap_or((endpoint, ""));
    let mut stream = TcpStream::connect(authority)?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    let target = format!("/{prefix}{}", path.trim_start_matches('/'));
    let auth = token
        .map(|value| format!("Authorization: Bearer {value}\r\n"))
        .unwrap_or_default();
    write!(
        stream,
        "GET {target} HTTP/1.1\r\nHost: {authority}\r\n{auth}Connection: close\r\n\r\n"
    )?;
    let mut response = String::new();
    let mut chunk = [0_u8; 4096];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(size) => {
                let bytes = chunk
                    .get(..size)
                    .ok_or_else(|| rejected("peer response length exceeded buffer"))?;
                response.push_str(&String::from_utf8_lossy(bytes))
            }
            Err(_error) if !response.is_empty() => break,
            Err(error) => return Err(error.into()),
        }
    }
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| rejected("peer returned malformed HTTP response"))?;
    if !head.starts_with("HTTP/1.1 200") && !head.starts_with("HTTP/1.0 200") {
        return Err(rejected("peer request was rejected or unavailable"));
    }
    Ok(body.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{PeerRecord, PeerRegistry, PeerRegistryDto, PeerRegistryEntryDto};
    use crate::error::CoordinationError;
    use enforcer_domain::coordination_types::CoordinationLedgerRoot;
    use enforcer_domain::coordination_types::{
        CoordinationPeerName, CoordinationPeerTokenEnv, CoordinationPeerUrl,
    };
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpListener};
    use std::sync::{Mutex, MutexGuard, OnceLock};

    static NETWORK_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn network_test_lock() -> Result<MutexGuard<'static, ()>, Box<dyn std::error::Error>> {
        NETWORK_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .map_err(|error| format!("peer network test lock poisoned: {error}").into())
    }

    fn peer_server(
        responses: Vec<String>,
        required_token: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        let token = required_token.map(str::to_owned);
        std::thread::spawn(move || {
            for response in responses {
                let Ok((mut socket, _)) = listener.accept() else {
                    return;
                };
                let mut request_bytes = Vec::new();
                let mut chunk = [0_u8; 512];
                while !request_bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                    let Ok(read) = socket.read(&mut chunk) else {
                        return;
                    };
                    if read == 0 {
                        break;
                    }
                    request_bytes.extend_from_slice(&chunk[..read]);
                }
                let request = String::from_utf8_lossy(&request_bytes);
                let permitted = token.as_ref().is_none_or(|value| {
                    request.contains(&format!("Authorization: Bearer {value}"))
                });
                let status = if permitted {
                    "200 OK"
                } else {
                    "401 Unauthorized"
                };
                let body = if permitted {
                    response
                } else {
                    "unauthorized".to_owned()
                };
                if write!(
                    socket,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .is_err()
                {
                    return;
                }
                if socket.flush().is_err() {
                    return;
                }
                let _ = socket.shutdown(Shutdown::Write);
            }
        });
        Ok(format!("http://{address}"))
    }

    #[test]
    fn peer_registry_dto_roundtrip_preserves_typed_peer_contract(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let registry = PeerRegistry {
            peers: vec![PeerRecord {
                name: CoordinationPeerName::parse("left")?,
                url: CoordinationPeerUrl::parse("http://127.0.0.1:8787")?,
                token_env: Some(CoordinationPeerTokenEnv::parse(
                    "LEDGER_PEER_TOKEN".to_owned(),
                )?),
            }],
        };
        let encoded = serde_json::to_string(&PeerRegistryDto::from(&registry))?;
        let decoded: PeerRegistryDto = serde_json::from_str(&encoded)?;
        let restored: PeerRegistry = decoded.try_into()?;
        assert_eq!(restored.peers[0].name.as_str(), "left");
        assert_eq!(
            restored.peers[0]
                .token_env
                .as_ref()
                .map(CoordinationPeerTokenEnv::as_str),
            Some("LEDGER_PEER_TOKEN")
        );
        Ok(())
    }

    #[test]
    fn peer_registry_dto_wire_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let input: PeerRegistryDto = PeerRegistryDto {
            peers: vec![PeerRegistryEntryDto {
                name: "left".to_owned(),
                url: "http://127.0.0.1:8787".to_owned(),
                token_env: Some("LEDGER_PEER_TOKEN".to_owned()),
                mode: Some("pull".to_owned()),
            }],
        };
        let encoded = serde_json::to_string(&input)?;
        let output: PeerRegistryDto = serde_json::from_str(&encoded)?;
        assert_eq!(output, input);
        Ok(())
    }

    #[test]
    fn peer_registry_entry_dto_wire_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let input: PeerRegistryEntryDto = PeerRegistryEntryDto {
            name: "left".to_owned(),
            url: "http://127.0.0.1:8787".to_owned(),
            token_env: None,
            mode: Some("pull".to_owned()),
        };
        let encoded = serde_json::to_string(&input)?;
        let output: PeerRegistryEntryDto = serde_json::from_str(&encoded)?;
        assert_eq!(output, input);
        Ok(())
    }

    #[test]
    fn peer_registry_dto_rejects_malformed_peer_url() -> Result<(), Box<dyn std::error::Error>> {
        let decoded: PeerRegistryDto =
            serde_json::from_str(r#"{"peers":[{"name":"left","url":"https://not-supported"}]}"#)?;
        let error = PeerRegistry::try_from(decoded)
            .err()
            .ok_or("malformed peer URL must be rejected")?;
        let decode = match error {
            CoordinationError::Decode(error) => error,
            other => return Err(format!("expected a typed decode error, got {other:?}").into()),
        };
        assert_eq!(decode.path, "coordinationPeerUrl");
        assert_eq!(decode.reason, "expected a non-empty http:// peer endpoint");
        Ok(())
    }

    #[test]
    fn sync_http_rejects_bad_bearer_token() -> Result<(), Box<dyn std::error::Error>> {
        let _network = network_test_lock()?;
        let ledger = tempfile::tempdir()?;
        let root = CoordinationLedgerRoot::parse(ledger.path())?;
        let endpoint = CoordinationPeerUrl::parse(&peer_server(
            vec![r#"{"streams":[]}"#.to_owned()],
            Some("correct"),
        )?)?;
        let error = super::sync_http(&root, &endpoint, Some("wrong"))
            .err()
            .ok_or("a bad bearer token must be rejected")?;
        match error {
            CoordinationError::Rejected(reason) => {
                assert_eq!(reason.as_str(), "peer request was rejected or unavailable")
            }
            other => return Err(format!("expected a typed rejection, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn sync_http_rejects_malformed_manifest() -> Result<(), Box<dyn std::error::Error>> {
        let _network = network_test_lock()?;
        let ledger = tempfile::tempdir()?;
        let root = CoordinationLedgerRoot::parse(ledger.path())?;
        let endpoint = CoordinationPeerUrl::parse(&peer_server(
            vec![r#"{"streams":["../escape.ndjson"]}"#.to_owned()],
            None,
        )?)?;
        let error = super::sync_http(&root, &endpoint, None)
            .err()
            .ok_or("a manifest with path traversal must be rejected")?;
        match error {
            CoordinationError::Rejected(reason) => {
                assert_eq!(
                    reason.as_str(),
                    "peer manifest contains an unsafe stream name"
                )
            }
            other => return Err(format!("expected a typed rejection, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn sync_lines_preserves_divergence_as_conflict_artifact(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ledger = tempfile::tempdir()?;
        std::fs::create_dir_all(ledger.path().join("streams"))?;
        std::fs::write(
            ledger.path().join("streams").join("node.a.ndjson"),
            "local\n",
        )?;
        let root = CoordinationLedgerRoot::parse(ledger.path())?;
        let result = super::sync_lines(
            &root,
            vec![("node.a.ndjson".to_owned(), vec!["remote".to_owned()])],
        )?;
        assert_eq!(result.imported, 0);
        assert_eq!(result.conflicts.len(), 1);
        assert!(ledger
            .path()
            .join("streams")
            .join(&result.conflicts[0])
            .exists());
        Ok(())
    }
}
