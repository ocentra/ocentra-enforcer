export function createFrameReader() {
  let inputBuffer = Buffer.alloc(0);
  return {
    push(chunk) {
      inputBuffer = Buffer.concat([inputBuffer, chunk]);
      return drainFrames(() => readFrameFromBuffer(inputBuffer), (nextBuffer) => {
        inputBuffer = nextBuffer;
      });
    },
  };
}

function drainFrames(readFrame, setBuffer) {
  const frames = [];
  while (true) {
    const frame = readFrame();
    if (frame === null) {
      return frames;
    }
    frames.push(frame.frame);
    setBuffer(frame.remainingBuffer);
  }
}

function readFrameFromBuffer(buffer) {
  return isContentLengthFrame(buffer)
    ? readContentLengthFrame(buffer)
    : readNdjsonFrame(buffer);
}

function isContentLengthFrame(buffer) {
  const prefix = buffer
    .slice(0, Math.min(buffer.length, 64))
    .toString("utf8")
    .trimStart();
  return prefix.toLowerCase().startsWith("content-length:");
}

function readNdjsonFrame(buffer) {
  const lineEnd = buffer.indexOf("\n");
  if (lineEnd === -1) {
    return null;
  }
  const body = buffer
    .slice(0, lineEnd)
    .toString("utf8")
    .replace(/\r$/u, "");
  return {
    frame: { body, framing: "ndjson" },
    remainingBuffer: buffer.slice(lineEnd + 1),
  };
}

function readContentLengthFrame(buffer) {
  const boundary = findHeaderBoundary(buffer);
  if (boundary === null) {
    return null;
  }
  const header = buffer.slice(0, boundary.headerEnd).toString("utf8");
  const contentLength = parseContentLength(header);
  const messageStart = boundary.headerEnd + boundary.separatorLength;
  const messageEnd = messageStart + contentLength;
  if (buffer.length < messageEnd) {
    return null;
  }
  return {
    frame: {
      body: buffer.slice(messageStart, messageEnd).toString("utf8"),
      framing: "content-length",
    },
    remainingBuffer: buffer.slice(messageEnd),
  };
}

function findHeaderBoundary(buffer) {
  const candidates = [
    { headerEnd: buffer.indexOf("\r\n\r\n"), separatorLength: 4 },
    { headerEnd: buffer.indexOf("\n\n"), separatorLength: 2 },
  ].filter((candidate) => candidate.headerEnd !== -1);
  if (candidates.length === 0) {
    return null;
  }
  return candidates.sort((left, right) => left.headerEnd - right.headerEnd)[0];
}

function parseContentLength(header) {
  const lengthMatch = /content-length:\s*(\d+)/iu.exec(header);
  if (!lengthMatch) {
    throw new Error("MCP frame missing Content-Length header.");
  }
  return Number(lengthMatch[1]);
}
