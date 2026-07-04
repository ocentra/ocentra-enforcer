//! Pure JSON-RPC framing + message types over MCP stdio.
//!
//! Ported (behavior-parity, no re-implementation drift) from the legacy
//! `.mjs` transport pair `mcp/rust-rules-mcp-transport-frames.mjs` +
//! `mcp/rust-rules-mcp-transport-messages.mjs`: dual framing, auto-detected
//! per message — an `Content-Length:` header block (LSP-style) OR a bare
//! NDJSON line — and the reply is always emitted in the SAME framing the
//! request arrived in.
//!
//! This module is deliberately I/O-free: [`FrameReader`] consumes bytes
//! already read from stdin and yields complete [`Frame`]s; encoding a
//! reply back to bytes is [`encode_frame`]. The actual `Read`/`Write`
//! against real stdio handles lives in [`crate::sink`], the ONE module in
//! this crate carrying the `print_stdout`/`print_stderr` allow.

use std::collections::VecDeque;

/// Which framing a message was read in (or should be replied in).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framing {
    /// `Content-Length: N\r\n\r\n<N bytes>` (LSP-style).
    ContentLength,
    /// A single line of JSON terminated by `\n`.
    Ndjson,
}

/// One complete, still-raw (unparsed) message body plus the framing it
/// arrived in.
#[derive(Debug, Clone)]
pub struct Frame {
    pub body: String,
    pub framing: Framing,
}

/// Incremental byte-buffer framer: feed it chunks via [`FrameReader::push`],
/// drain zero or more complete [`Frame`]s per call. Mirrors
/// `createFrameReader`/`drainFrames` from the legacy `.mjs` pair exactly:
/// framing is detected PER FRAME (a NDJSON line does not commit the whole
/// stream to NDJSON), and a frame lacking enough buffered bytes yet simply
/// waits for the next `push`.
#[derive(Debug, Default)]
pub struct FrameReader {
    buffer: Vec<u8>,
}

impl FrameReader {
    /// A fresh reader with an empty buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed newly-read bytes and drain every frame now complete.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<Frame> {
        self.buffer.extend_from_slice(chunk);
        let mut out = Vec::new();
        while let Some((frame, consumed)) = read_frame(&self.buffer) {
            out.push(frame);
            self.buffer.drain(0..consumed);
        }
        out
    }
}

/// Try to read exactly one frame from the front of `buffer`. Returns the
/// frame plus how many bytes it consumed, or `None` if more bytes are
/// needed.
fn read_frame(buffer: &[u8]) -> Option<(Frame, usize)> {
    if is_content_length_prefix(buffer) {
        read_content_length_frame(buffer)
    } else {
        read_ndjson_frame(buffer)
    }
}

fn is_content_length_prefix(buffer: &[u8]) -> bool {
    let probe_len = buffer.len().min(64);
    let probe = String::from_utf8_lossy(&buffer[..probe_len]);
    probe
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("content-length:")
}

fn read_ndjson_frame(buffer: &[u8]) -> Option<(Frame, usize)> {
    let newline_at = buffer.iter().position(|&b| b == b'\n')?;
    let line = &buffer[..newline_at];
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    let body = String::from_utf8_lossy(line).into_owned();
    Some((
        Frame {
            body,
            framing: Framing::Ndjson,
        },
        newline_at + 1,
    ))
}

fn read_content_length_frame(buffer: &[u8]) -> Option<(Frame, usize)> {
    let (header_end, separator_len) = find_header_boundary(buffer)?;
    let header = String::from_utf8_lossy(&buffer[..header_end]);
    let content_length = parse_content_length(&header)?;
    let message_start = header_end + separator_len;
    let message_end = message_start.checked_add(content_length)?;
    if buffer.len() < message_end {
        return None;
    }
    let body = String::from_utf8_lossy(&buffer[message_start..message_end]).into_owned();
    Some((
        Frame {
            body,
            framing: Framing::ContentLength,
        },
        message_end,
    ))
}

/// Find the header/body boundary, preferring `\r\n\r\n`, falling back to
/// `\n\n` (matches the legacy `.mjs` candidate order).
fn find_header_boundary(buffer: &[u8]) -> Option<(usize, usize)> {
    if let Some(at) = find_subslice(buffer, b"\r\n\r\n") {
        return Some((at, 4));
    }
    find_subslice(buffer, b"\n\n").map(|at| (at, 2))
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn parse_content_length(header: &str) -> Option<usize> {
    header
        .lines()
        .find_map(|line| line.split_once(':'))
        .filter(|(key, _)| key.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
}

/// Encode a JSON body back into wire bytes for the given [`Framing`].
/// Mirrors the legacy `send`/`sendResult`/`sendError` encoding exactly.
pub fn encode_frame(body: &str, framing: Framing) -> Vec<u8> {
    match framing {
        Framing::Ndjson => {
            let mut out = body.as_bytes().to_vec();
            out.push(b'\n');
            out
        }
        Framing::ContentLength => {
            let header = format!("Content-Length: {}\r\n\r\n", body.len());
            let mut out = header.into_bytes();
            out.extend_from_slice(body.as_bytes());
            out
        }
    }
}

/// A parsed JSON-RPC request/notification. `id` absent + `method` starting
/// with `notifications/` marks a fire-and-forget notification (never
/// replied to), matching `isNotification` in the legacy `.mjs` transport.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RpcMessage {
    #[serde(default)]
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

impl RpcMessage {
    /// True for a fire-and-forget notification (no reply expected).
    pub fn is_notification(&self) -> bool {
        self.id.is_none() && self.method.starts_with("notifications/")
    }
}

/// A JSON-RPC success reply.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RpcResult {
    pub jsonrpc: &'static str,
    pub id: serde_json::Value,
    pub result: serde_json::Value,
}

impl RpcResult {
    pub fn new(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result,
        }
    }
}

/// A JSON-RPC error reply. Standard JSON-RPC error codes: `-32700` parse
/// error, `-32601` method not found, `-32603` internal error (mirrors the
/// legacy `.mjs` transport's exact codes).
#[derive(Debug, Clone, serde::Serialize)]
pub struct RpcError {
    pub jsonrpc: &'static str,
    pub id: serde_json::Value,
    pub error: RpcErrorBody,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RpcErrorBody {
    pub code: i64,
    pub message: String,
}

impl RpcError {
    pub const PARSE_ERROR: i64 = -32700;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INTERNAL_ERROR: i64 = -32603;

    pub fn new(id: serde_json::Value, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            error: RpcErrorBody {
                code,
                message: message.into(),
            },
        }
    }
}

/// A queue-based helper used by tests (and available to [`crate::sink`]) to
/// drain a byte stream in fixed-size chunks, mimicking chunked stdin reads
/// without depending on real I/O.
pub fn chunk_bytes(bytes: &[u8], chunk_size: usize) -> VecDeque<Vec<u8>> {
    let chunk_size = chunk_size.max(1);
    bytes.chunks(chunk_size).map(<[u8]>::to_vec).collect()
}

#[cfg(test)]
mod tests {
    use super::{encode_frame, FrameReader, Framing, RpcMessage};

    #[test]
    fn ndjson_frame_round_trips() -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = FrameReader::new();
        let frames = reader.push(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].framing, Framing::Ndjson);
        let message: RpcMessage = serde_json::from_str(&frames[0].body)?;
        assert_eq!(message.method, "ping");

        let encoded = encode_frame("{\"ok\":true}", Framing::Ndjson);
        assert_eq!(encoded, b"{\"ok\":true}\n");
        Ok(())
    }

    #[test]
    fn content_length_frame_round_trips() -> Result<(), Box<dyn std::error::Error>> {
        let body = "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}";
        let wire = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let mut reader = FrameReader::new();
        let frames = reader.push(wire.as_bytes());
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].framing, Framing::ContentLength);
        assert_eq!(frames[0].body, body);

        let encoded = encode_frame("{\"ok\":true}", Framing::ContentLength);
        let encoded_str = String::from_utf8(encoded)?;
        assert_eq!(encoded_str, "Content-Length: 11\r\n\r\n{\"ok\":true}");
        Ok(())
    }

    #[test]
    fn partial_frame_waits_for_more_bytes() {
        // Full body `{"partial":true}` is 16 bytes; declare Content-Length
        // 16 and split the wire bytes mid-body so the reader must wait for
        // the remainder before yielding a frame.
        let body = "{\"partial\":true}";
        assert_eq!(body.len(), 16);
        let wire = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let wire_bytes = wire.as_bytes();
        let split_at = wire_bytes.len() - 1; // withhold the final `}`
        let mut reader = FrameReader::new();
        let frames = reader.push(&wire_bytes[..split_at]);
        assert!(frames.is_empty(), "incomplete body must not yield a frame");
        let frames = reader.push(&wire_bytes[split_at..]);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].body, body);
    }

    #[test]
    fn chunked_ndjson_across_multiple_pushes_still_frames_correctly() {
        let raw = b"{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"ping\"}\n";
        let mut reader = FrameReader::new();
        let mut collected = Vec::new();
        for chunk in super::chunk_bytes(raw, 5) {
            collected.extend(reader.push(&chunk));
        }
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].framing, Framing::Ndjson);
    }

    #[test]
    fn two_ndjson_frames_in_one_push_both_drain() {
        let mut reader = FrameReader::new();
        let frames = reader.push(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"a\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"b\"}\n");
        assert_eq!(frames.len(), 2);
    }

    #[test]
    fn malformed_json_body_is_a_fail_fixture_for_the_router_layer() {
        // transport.rs itself does not parse JSON payload semantics beyond
        // RpcMessage's shape; a malformed body still yields ONE frame (the
        // framing layer's job), and the router/sink layer is responsible
        // for turning the JSON parse failure into a -32700 RpcError. This
        // fixture documents that boundary.
        let mut reader = FrameReader::new();
        let frames = reader.push(b"not json at all\n");
        assert_eq!(frames.len(), 1);
        assert!(serde_json::from_str::<RpcMessage>(&frames[0].body).is_err());
    }

    #[test]
    fn notification_detection_matches_legacy_semantics() -> Result<(), Box<dyn std::error::Error>> {
        let notif: RpcMessage = serde_json::from_str("{\"method\":\"notifications/initialized\"}")?;
        assert!(notif.is_notification());
        let request: RpcMessage = serde_json::from_str("{\"id\":1,\"method\":\"ping\"}")?;
        assert!(!request.is_notification());
        Ok(())
    }
}
