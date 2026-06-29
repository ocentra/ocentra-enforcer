#!/usr/bin/env node
/*
 * Minimal MCP stdio adapter for the Rust Rules hard gate.
 * The rule engine lives in scripts/rust-rules.mjs; this file only exposes it.
 */
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import {
  decodeDoctorToolArguments,
  decodeExplainToolArguments,
  decodeRouteRequest,
  decodeRuleRegistry,
  decodeScanToolArguments,
} from '../schemas/effect/enforcer-schemas.mjs';

const MCP_PROTOCOL_VERSION = '2025-06-18';
const SERVER_ROOT = path.resolve(path.join(path.dirname(fileURLToPath(import.meta.url)), '..'));
const CLI_PATH = path.join(SERVER_ROOT, 'scripts', 'rust-rules.mjs');
const RULE_REGISTRY_PATH = path.join(SERVER_ROOT, 'rules', 'rules.json');
const PACKAGE_JSON = JSON.parse(fs.readFileSync(path.join(SERVER_ROOT, 'package.json'), 'utf8'));

const SCOPE_SCHEMA = {
  type: 'string',
  enum: ['workspace', 'files', 'crate', 'diff'],
  description: 'Validation scope. Defaults to workspace unless files, crateName, or base/head imply a narrower scope.',
};

const COMMON_INPUT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  properties: {
    root: {
      type: 'string',
      description: 'Target repository root. Defaults to the MCP server working directory.',
    },
    configPath: {
      type: 'string',
      description: 'Optional Ocentra Enforcer config path. Relative paths resolve against root.',
    },
    profile: {
      type: 'string',
      description: 'Optional named pack profile such as strict or ocentra-parent. Ignored when configPath is provided.',
    },
    scope: SCOPE_SCHEMA,
    files: {
      type: 'array',
      items: { type: 'string' },
      description: 'Files or directories for files scope.',
    },
    crateName: {
      type: 'string',
      description: 'Cargo package name for crate scope.',
    },
    base: {
      type: 'string',
      description: 'Base git ref for diff scope.',
    },
    head: {
      type: 'string',
      description: 'Head git ref for diff scope.',
    },
  },
};

const CANONICAL_TOOLS = [
  {
    name: 'ocentra_enforcer_route',
    description: 'Return compact indexed Ocentra Enforcer rule docs relevant to files, crate, scope, profile, or one rule ID.',
    inputSchema: {
      ...COMMON_INPUT_SCHEMA,
      properties: {
        ...COMMON_INPUT_SCHEMA.properties,
        ruleId: {
          type: 'string',
          description: 'Optional explicit rule ID such as RR-7.3. When provided, routes directly to that rule.',
        },
      },
    },
  },
  {
    name: 'ocentra_enforcer_scan',
    description: 'Run deterministic Rust source/config/dependency scanner by workspace, files, crate, or diff scope.',
    inputSchema: {
      ...COMMON_INPUT_SCHEMA,
      properties: {
        ...COMMON_INPUT_SCHEMA.properties,
        cargo: {
          type: 'boolean',
          description: 'When true, run cargo gates in addition to scanner checks.',
          default: false,
        },
      },
    },
  },
  {
    name: 'ocentra_enforcer_doctor',
    description: 'Check Ocentra Enforcer wiring for a target root/config/scope without changing files.',
    inputSchema: COMMON_INPUT_SCHEMA,
  },
  {
    name: 'ocentra_enforcer_explain',
    description: 'Explain one Ocentra Enforcer rule ID and give the docs anchor/fix hint.',
    inputSchema: {
      type: 'object',
      additionalProperties: false,
      required: ['ruleId'],
      properties: {
        ruleId: {
          type: 'string',
          description: 'Rule ID such as RR-7.3.',
        },
      },
    },
  },
];

const TOOLS = [
  ...CANONICAL_TOOLS,
  ...CANONICAL_TOOLS.map((tool) => ({
    ...tool,
    name: tool.name.replace('ocentra_enforcer_', 'rust_rules_'),
    description: `Legacy alias for ${tool.name}; kept for one Rust-pack compatibility release.`,
  })),
];

let inputBuffer = Buffer.alloc(0);

process.stdin.on('data', (chunk) => {
  inputBuffer = Buffer.concat([inputBuffer, chunk]);
  processInputFrames();
});

process.stdin.on('end', () => {
  process.exit(0);
});

function processInputFrames() {
  while (inputBuffer.length > 0) {
    const frame = readFrame();
    if (frame === null) return;
    handleRawMessage(frame);
  }
}

function readFrame() {
  const crlfHeaderEnd = inputBuffer.indexOf('\r\n\r\n');
  const lfHeaderEnd = inputBuffer.indexOf('\n\n');
  let headerEnd = -1;
  let separatorLength = 0;
  if (crlfHeaderEnd !== -1 && (lfHeaderEnd === -1 || crlfHeaderEnd < lfHeaderEnd)) {
    headerEnd = crlfHeaderEnd;
    separatorLength = 4;
  } else if (lfHeaderEnd !== -1) {
    headerEnd = lfHeaderEnd;
    separatorLength = 2;
  }
  if (headerEnd === -1) return null;

  const header = inputBuffer.slice(0, headerEnd).toString('utf8');
  const lengthMatch = /content-length:\s*(\d+)/iu.exec(header);
  if (!lengthMatch) {
    throw new Error('MCP frame missing Content-Length header.');
  }

  const contentLength = Number(lengthMatch[1]);
  const messageStart = headerEnd + separatorLength;
  const messageEnd = messageStart + contentLength;
  if (inputBuffer.length < messageEnd) return null;

  const body = inputBuffer.slice(messageStart, messageEnd).toString('utf8');
  inputBuffer = inputBuffer.slice(messageEnd);
  return body;
}

function handleRawMessage(raw) {
  let message;
  try {
    message = JSON.parse(raw);
  } catch (error) {
    sendError(null, -32700, `Parse error: ${error.message}`);
    return;
  }

  Promise.resolve()
    .then(() => handleMessage(message))
    .catch((error) => {
      if (message.id !== undefined) {
        sendError(message.id, -32603, error instanceof Error ? error.message : String(error));
      }
    });
}

async function handleMessage(message) {
  if (message.id === undefined && String(message.method ?? '').startsWith('notifications/')) return;

  switch (message.method) {
    case 'initialize':
      sendResult(message.id, {
        protocolVersion: message.params?.protocolVersion ?? MCP_PROTOCOL_VERSION,
        capabilities: { tools: {} },
        serverInfo: {
          name: PACKAGE_JSON.name,
          version: PACKAGE_JSON.version,
        },
      });
      return;
    case 'ping':
      sendResult(message.id, {});
      return;
    case 'tools/list':
      sendResult(message.id, { tools: TOOLS });
      return;
    case 'tools/call':
      sendResult(message.id, callTool(message.params ?? {}));
      return;
    case 'shutdown':
      sendResult(message.id, null);
      return;
    default:
      sendError(message.id, -32601, `Unknown method: ${message.method}`);
  }
}

function callTool(params) {
  const name = normalizeToolName(params.name);
  const args = params.arguments ?? {};
  try {
    if (name === 'ocentra_enforcer_route') {
      return {
        isError: false,
        content: [{ type: 'text', text: JSON.stringify(routeRules(decodeRouteRequest(args)), null, 2) }],
      };
    }
    if (name === 'ocentra_enforcer_scan') {
      const decoded = decodeScanToolArguments(args);
      return runCli(decoded.cargo ? 'cargo' : 'scan', decoded);
    }
    if (name === 'ocentra_enforcer_doctor') {
      return runCli('doctor', decodeDoctorToolArguments(args));
    }
    if (name === 'ocentra_enforcer_explain') {
      return runCli('explain', decodeExplainToolArguments(args));
    }
    return toolError(`Unknown tool: ${params.name}`);
  } catch (error) {
    return toolError(error instanceof Error ? error.message : String(error));
  }
}

function normalizeToolName(name) {
  return String(name ?? '').replace(/^rust_rules_/u, 'ocentra_enforcer_');
}

function toolError(message) {
  return {
    isError: true,
    content: [{ type: 'text', text: message }],
  };
}

function routeRules(args) {
  const root = path.resolve(args.root ?? process.cwd());
  const registry = loadRuleRegistry();
  const profileName = resolveProfileName(root, args);
  const explicitRuleId = args.ruleId?.toUpperCase() ?? null;
  const families = explicitRuleId ? [] : routeFamilies(args);
  const rules = explicitRuleId
    ? registry.rules.filter((rule) => rule.id === explicitRuleId)
    : registry.rules.filter((rule) => families.has(rule.family));
  const docs = uniqueSorted(rules.map((rule) => rule.doc));

  return {
    ok: true,
    productName: registry.productName,
    profileName,
    index: 'rules/INDEX.md',
    scope: describeRouteScope(args),
    docs,
    rules: rules.map((rule) => ({
      id: rule.id,
      family: rule.family,
      severity: rule.severity,
      doc: rule.doc,
      validator: rule.validator,
    })),
  };
}

function loadRuleRegistry() {
  return decodeRuleRegistry(JSON.parse(fs.readFileSync(RULE_REGISTRY_PATH, 'utf8')));
}

function resolveProfileName(root, args) {
  if (args.configPath) {
    const configPath = path.isAbsolute(args.configPath) ? args.configPath : path.join(root, args.configPath);
    if (!fs.existsSync(configPath)) return 'custom';
    const parsed = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    return parsed.profileName ?? 'custom';
  }
  return args.profile ?? 'strict';
}

function routeFamilies(args) {
  if (args.scope === 'crate' || args.scope === 'workspace') {
    return new Set(['source', 'domain', 'imports-modules', 'async-runtime', 'toolchain-cargo', 'dependencies']);
  }

  const files = Array.isArray(args.files) ? args.files : [];
  const families = new Set();
  for (const file of files) {
    for (const family of routeFamiliesForFile(file)) families.add(family);
  }
  return families;
}

function routeFamiliesForFile(file) {
  const normalized = file.split(/[\\/]+/u).pop() ?? file;
  if (file.endsWith('.rs')) return ['source', 'domain', 'imports-modules', 'async-runtime'];
  if (normalized === 'Cargo.toml') return ['toolchain-cargo', 'dependencies'];
  if (normalized === 'Cargo.lock' || normalized === 'deny.toml') return ['dependencies'];
  if (normalized === 'rust-toolchain.toml' || normalized === 'clippy.toml' || normalized === 'rustfmt.toml') {
    return ['toolchain-cargo'];
  }
  return [];
}

function describeRouteScope(args) {
  if (args.ruleId) return { mode: 'rule', ruleId: args.ruleId.toUpperCase() };
  if (args.scope === 'crate') return { mode: 'crate', crateName: args.crateName ?? null };
  if (args.scope === 'diff') return { mode: 'diff', base: args.base ?? null, head: args.head ?? null, files: args.files ?? [] };
  if (args.scope === 'workspace') return { mode: 'workspace' };
  return { mode: 'files', files: args.files ?? [] };
}

function runCli(command, args) {
  if (command === 'explain') {
    return runCliProcess([CLI_PATH, 'explain', args.ruleId, '--json'], process.cwd());
  }

  const root = path.resolve(args.root ?? process.cwd());
  const cliArgs = [CLI_PATH, command, '--root', root, '--json'];
  const configPath = resolveConfigPath(root, args);
  if (configPath) {
    cliArgs.push('--config', configPath);
  }
  cliArgs.push(...scopeArgs(args));

  return runCliProcess(cliArgs, root);
}

function runCliProcess(cliArgs, cwd) {
  const result = spawnSync(process.execPath, cliArgs, {
    cwd,
    encoding: 'utf8',
    shell: false,
  });

  const stdout = result.stdout?.trim() ?? '';
  const stderr = result.stderr?.trim() ?? '';
  const text = stdout || JSON.stringify({ ok: false, status: result.status, stderr }, null, 2);
  return {
    isError: (result.status ?? 1) !== 0,
    content: [
      {
        type: 'text',
        text,
      },
    ],
  };
}

function resolveConfigPath(root, args) {
  if (args.configPath) {
    return path.isAbsolute(args.configPath) ? args.configPath : path.join(root, args.configPath);
  }

  const profile = args.profile ?? null;
  if (profile === null || profile === '' || profile === 'strict') return null;
  if (!/^[A-Za-z0-9][A-Za-z0-9_.-]*$/u.test(profile)) {
    throw new Error(`Invalid profile name: ${profile}`);
  }

  const profilePath = path.join(SERVER_ROOT, 'profiles', `${profile}.json`);
  if (!fs.existsSync(profilePath)) {
    throw new Error(`Unknown Ocentra Enforcer profile "${profile}". Expected ${profilePath}.`);
  }
  return profilePath;
}

function uniqueSorted(values) {
  return [...new Set(values)].sort((a, b) => a.localeCompare(b));
}

function scopeArgs(args) {
  const inferredScope =
    args.scope ??
    (Array.isArray(args.files) && args.files.length > 0
      ? 'files'
      : args.crateName
        ? 'crate'
        : args.base || args.head
          ? 'diff'
          : 'workspace');

  if (inferredScope === 'files') {
    if (!Array.isArray(args.files) || args.files.length === 0) throw new Error('files scope requires files.');
    return ['--files', ...args.files];
  }
  if (inferredScope === 'crate') {
    if (!args.crateName) throw new Error('crate scope requires crateName.');
    return ['--crate', args.crateName];
  }
  if (inferredScope === 'diff') {
    if (!args.base || !args.head) throw new Error('diff scope requires base and head.');
    return ['--base', args.base, '--head', args.head];
  }
  return ['--workspace'];
}

function sendResult(id, result) {
  send({ jsonrpc: '2.0', id, result });
}

function sendError(id, code, message) {
  send({ jsonrpc: '2.0', id, error: { code, message } });
}

function send(message) {
  const body = JSON.stringify(message);
  process.stdout.write(`Content-Length: ${Buffer.byteLength(body, 'utf8')}\r\n\r\n${body}`);
}
