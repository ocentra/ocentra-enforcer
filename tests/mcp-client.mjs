import assert from "node:assert/strict";

export function createMcpClient(server, framing = "content-length") {
  const state = createClientState(server);
  attachReaders(state, framing);
  return {
    request(id, method, params) {
      sendFrame(server, { jsonrpc: "2.0", id, method, params }, framing);
      return waitForResponse(state, id);
    },
    notify(method, params) {
      sendFrame(server, { jsonrpc: "2.0", method, params }, framing);
    },
  };
}

function createClientState(server) {
  return {
    server,
    output: Buffer.alloc(0),
    received: new Map(),
    waiters: new Map(),
    stderr: "",
  };
}

function attachReaders(state, framing) {
  state.server.stderr.on("data", (chunk) => {
    state.stderr += chunk.toString("utf8");
  });
  state.server.stdout.on("data", (chunk) => {
    state.output = Buffer.concat([state.output, chunk]);
    drainFrames(state, framing);
  });
}

function drainFrames(state, framing) {
  while (state.output.length > 0) {
    const frame = readFrame(state, framing);
    if (frame === null) return;
    receiveMessage(state, JSON.parse(frame));
  }
}

function receiveMessage(state, message) {
  if (message.id === undefined) return;
  const waiter = state.waiters.get(message.id);
  if (waiter === undefined) {
    state.received.set(message.id, message);
    return;
  }
  state.waiters.delete(message.id);
  waiter.resolve(message);
}

function waitForResponse(state, id) {
  if (state.received.has(id)) {
    const message = state.received.get(id);
    state.received.delete(id);
    return Promise.resolve(message);
  }
  return new Promise((resolve, reject) => {
    registerWaiter(state, id, resolve, reject);
  });
}

function registerWaiter(state, id, resolve, reject) {
  // TIMER-JUSTIFICATION: MCP protocol tests need a bounded child-process response timeout.
  const timeout = setTimeout(() => {
    state.waiters.delete(id);
    reject(new Error(`Timed out waiting for MCP response ${id}. stderr=${state.stderr}`));
  }, 30000);
  state.waiters.set(id, {
    resolve(message) {
      clearTimeout(timeout);
      resolve(message);
    },
  });
}

function readFrame(state, framing) {
  if (framing === "ndjson") return readNdjsonFrame(state);
  return readContentLengthFrame(state);
}

function readNdjsonFrame(state) {
  const lineEnd = state.output.indexOf("\n");
  if (lineEnd === -1) return null;
  const body = state.output
    .slice(0, lineEnd)
    .toString("utf8")
    .replace(/\r$/u, "");
  state.output = state.output.slice(lineEnd + 1);
  return body;
}

function readContentLengthFrame(state) {
  const headerEnd = state.output.indexOf("\r\n\r\n");
  if (headerEnd === -1) return null;
  const header = state.output.slice(0, headerEnd).toString("utf8");
  const contentLength = parseContentLength(header);
  const bodyStart = headerEnd + 4;
  const bodyEnd = bodyStart + contentLength;
  if (state.output.length < bodyEnd) return null;
  const body = state.output.slice(bodyStart, bodyEnd).toString("utf8");
  state.output = state.output.slice(bodyEnd);
  return body;
}

function parseContentLength(header) {
  const lengthMatch = /content-length:\s*(\d+)/iu.exec(header);
  assert.ok(lengthMatch, `missing Content-Length in ${header}`);
  return Number(lengthMatch[1]);
}

function sendFrame(server, message, framing) {
  server.stdin.write(encodeFrame(message, framing));
}

function encodeFrame(message, framing) {
  const body = JSON.stringify(message);
  if (framing === "ndjson") return `${body}\n`;
  return `Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`;
}
