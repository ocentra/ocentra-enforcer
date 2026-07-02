import { createFrameReader } from "./rust-rules-mcp-transport-frames.mjs";
import { createMessageSession } from "./rust-rules-mcp-transport-messages.mjs";

export function startMcpStdioServer({ callTool, packageJson, tools }) {
  const frameReader = createFrameReader();
  const messageSession = createMessageSession({
    callTool,
    packageJson,
    sendError,
    sendResult,
    tools,
  });

  process.stdin.on("data", (chunk) => {
    for (const frame of frameReader.push(chunk)) {
      if (frame.body.trim().length > 0) {
        messageSession.handleRawMessage(frame.body, frame.framing);
      }
    }
  });

  process.stdin.on("end", () => {
    process.exit(0);
  });

  function sendResult(id, result, framing) {
    send({ jsonrpc: "2.0", id, result }, framing);
  }

  function sendError(id, code, message, framing) {
    send({ jsonrpc: "2.0", id, error: { code, message } }, framing);
  }

  function send(message, framing = "content-length") {
    const body = JSON.stringify(message);
    if (framing === "ndjson") {
      process.stdout.write(`${body}\n`);
    } else {
      process.stdout.write(
        `Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`,
      );
    }
  }
}
