export function createMessageSession({
  callTool,
  packageJson,
  sendError,
  sendResult,
  tools,
}) {
  return {
    handleRawMessage: createRawMessageHandler({
      handlers: createMessageHandlers(callTool, packageJson, tools),
      sendError,
      sendResult,
    }),
  };
}

function createRawMessageHandler({ handlers, sendError, sendResult }) {
  return function handleRawMessage(raw, framing) {
    const message = parseMessage(raw, framing, sendError);
    if (!message) {
      return;
    }
    Promise.resolve()
      .then(() => handleMessage(message, framing, handlers, sendError, sendResult))
      .catch((error) => {
        reportMessageFailure(error, framing, message, sendError);
      });
  };
}

function parseMessage(raw, framing, sendError) {
  try {
    return JSON.parse(raw);
  } catch (error) {
    sendError(null, -32700, `Parse error: ${error.message}`, framing);
    return null;
  }
}

async function handleMessage(message, framing, handlers, sendError, sendResult) {
  if (isNotification(message)) {
    return;
  }
  const handler = handlers[message.method] ?? null;
  if (!handler) {
    sendError(message.id, -32601, `Unknown method: ${message.method}`, framing);
    return;
  }
  sendResult(message.id, await handler(message), framing);
}

function createMessageHandlers(callTool, packageJson, tools) {
  return {
    initialize: (message) => initializeResult(message, packageJson),
    ping: emptyObject,
    "tools/list": () => ({ tools }),
    "tools/call": (message) => callTool(message.params ?? {}),
    "resources/list": emptyResources,
    "resources/templates/list": emptyResourceTemplates,
    "prompts/list": emptyPrompts,
    shutdown: returnNull,
  };
}

function initializeResult(message, packageJson) {
  return {
    protocolVersion: message.params?.protocolVersion ?? "2024-11-05",
    capabilities: { tools: {} },
    serverInfo: serverInfo(packageJson),
  };
}

function serverInfo(packageJson) {
  return {
    name: packageJson.name,
    version: packageJson.version,
  };
}

function emptyObject() {
  return {};
}

function emptyResources() {
  return { resources: [] };
}

function emptyResourceTemplates() {
  return { resourceTemplates: [] };
}

function emptyPrompts() {
  return { prompts: [] };
}

function returnNull() {
  return null;
}

function reportMessageFailure(error, framing, message, sendError) {
  if (message.id !== undefined) {
    sendError(
      message.id,
      -32603,
      error instanceof Error ? error.message : String(error),
      framing,
    );
  }
}

function isNotification(message) {
  return (
    message.id === undefined &&
    String(message.method ?? "").startsWith("notifications/")
  );
}
