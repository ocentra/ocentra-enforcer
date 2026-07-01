#!/usr/bin/env node
import path from 'node:path';
import process from 'node:process';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const PACK_ROOT = path.resolve(path.join(path.dirname(SCRIPT_PATH), '..'));
const SERVER_PATH = path.join(PACK_ROOT, 'mcp', 'ocentra-enforcer-mcp.mjs');

const args = parseArgs(process.argv.slice(2));
const server = spawn(process.execPath, [SERVER_PATH], {
  cwd: PACK_ROOT,
  stdio: ['pipe', 'pipe', 'pipe'],
});

let stderr = '';
server.stderr.on('data', (chunk) => {
  stderr += chunk.toString('utf8');
});

const client = createClient(server, args.framing);

try {
  const initialized = await client.request('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'ocentra-enforcer-mcp-smoke', version: '0.1.0' },
  });
  client.notify('notifications/initialized', {});

  const tools = await client.request('tools/list', {});
  const mcpStatusResult = await client.request('tools/call', {
    name: 'ocentra_enforcer_mcp_status',
    arguments: {},
  });
  const mcpStatus = JSON.parse(mcpStatusResult.result.content[0].text);
  if (mcpStatus.stale) {
    throw new Error(`MCP server is stale; restart Codex/MCP. Changed files: ${mcpStatus.changedFiles.map((file) => file.path).join(', ')}`);
  }
  const route = await client.request('tools/call', {
    name: 'ocentra_enforcer_route',
    arguments: {
      root: path.resolve(args.root),
      profile: args.profile,
      scope: 'files',
      files: [args.file],
    },
  });

  const routeReport = JSON.parse(route.result.content[0].text);
  const toolNames = tools.result.tools.map((tool) => tool.name).sort();
  const requiredTools = [
    'ocentra_enforcer_doctor',
    'ocentra_enforcer_explain',
    'ocentra_enforcer_check',
    'ocentra_enforcer_mcp_status',
    'ocentra_enforcer_route',
    'ocentra_enforcer_scan',
    'ocentra_enforcer_run',
    'ocentra_enforcer_run_status',
    'ocentra_enforcer_diagnostics',
    'ocentra_enforcer_last_failure',
    'ocentra_enforcer_artifact',
    'ocentra_enforcer_prune_runs',
    'ocentra_enforcer_reset_runs',
    'ocentra_enforcer_proof_route',
    'ocentra_enforcer_proof_run',
    'ocentra_enforcer_proof_status',
    'ocentra_enforcer_proof_inventory',
    'ocentra_enforcer_proof_import_legacy',
    'ocentra_enforcer_proof_parity',
    'ocentra_enforcer_proof_claim',
    'ocentra_enforcer_proof_last_failure',
    'ocentra_enforcer_proof_diagnostics',
    'ocentra_enforcer_proof_artifact',
    'ocentra_enforcer_proof_reset',
    'ocentra_enforcer_proof_prune',
    'ocentra_enforcer_proof_export',
    'ocentra_enforcer_coordination_init',
    'ocentra_enforcer_coordination_health',
    'ocentra_enforcer_coordination_presence',
    'ocentra_enforcer_coordination_index',
    'ocentra_enforcer_coordination_streams',
    'ocentra_enforcer_coordination_sync',
    'ocentra_enforcer_coordination_peer',
    'ocentra_enforcer_coordination_ensure',
    'ocentra_enforcer_coordination_compact',
    'ocentra_enforcer_coordination_notify',
    'ocentra_enforcer_coordination_mail',
    'ocentra_enforcer_coordination_inbox',
    'ocentra_enforcer_coordination_claim',
    'ocentra_enforcer_coordination_release',
    'ocentra_enforcer_coordination_closeout',
    'ocentra_enforcer_coordination_repair',
    'ocentra_enforcer_coordination_guard',
    'ocentra_enforcer_coordination_report',
    'ocentra_enforcer_coordination_message',
    'ocentra_enforcer_coordination_workers',
    'ocentra_enforcer_coordination_tasks',
  ];
  const missingTools = requiredTools.filter((tool) => !toolNames.includes(tool));
  if (missingTools.length > 0) {
    throw new Error(`MCP server is missing required tools: ${missingTools.join(', ')}`);
  }

  console.log(
    JSON.stringify(
      {
        ok: true,
        serverInfo: initialized.result.serverInfo,
        requiredTools,
        mcpStatus: {
          stale: mcpStatus.stale,
          writeCompatible: mcpStatus.writeCompatible,
          digest: mcpStatus.current.digest,
          startedAt: mcpStatus.startedAt,
        },
        legacyAliasesPresent: ['rust_rules_route', 'rust_rules_scan', 'rust_rules_doctor', 'rust_rules_explain', 'rust_rules_check'].every((tool) =>
          toolNames.includes(tool)
        ),
        route: {
          profileName: routeReport.profileName,
          docs: routeReport.docs,
          ruleCount: routeReport.rules.length,
        },
      },
      null,
      2
    )
  );
  await client.request('shutdown', {});
  server.kill();
  process.exit(0);
} catch (error) {
  server.kill();
  console.error(`MCP smoke failed: ${error instanceof Error ? error.message : String(error)}`);
  if (stderr.trim()) console.error(stderr.trim());
  process.exit(1);
}

function parseArgs(argv) {
  const parsed = {
    root: process.cwd(),
    profile: 'strict',
    file: 'Cargo.toml',
    framing: 'content-length',
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--root') parsed.root = argv[++i] ?? parsed.root;
    else if (arg === '--profile') parsed.profile = argv[++i] ?? parsed.profile;
    else if (arg === '--file') parsed.file = argv[++i] ?? parsed.file;
    else if (arg === '--framing') parsed.framing = argv[++i] ?? parsed.framing;
    else if (arg === '--help' || arg === '-h') {
      console.log('Usage: node scripts/mcp-smoke.mjs --root <target-repo> --profile <profile> --file <path> [--framing content-length|ndjson]');
      process.exit(0);
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }
  if (!['content-length', 'ndjson'].includes(parsed.framing)) throw new Error(`Unknown MCP framing: ${parsed.framing}`);
  return parsed;
}

function createClient(child, framing) {
  let nextId = 1;
  let output = Buffer.alloc(0);
  const received = new Map();
  const waiters = new Map();

  child.stdout.on('data', (chunk) => {
    output = Buffer.concat([output, chunk]);
    while (output.length > 0) {
      const frame = readFrame();
      if (frame === null) return;
      const message = JSON.parse(frame);
      if (message.id !== undefined && waiters.has(message.id)) {
        const waiter = waiters.get(message.id);
        waiters.delete(message.id);
        waiter.resolve(message);
      } else if (message.id !== undefined) {
        received.set(message.id, message);
      }
    }
  });

  return {
    request(method, params) {
      const id = nextId;
      nextId += 1;
      child.stdin.write(encodeFrame({ jsonrpc: '2.0', id, method, params }, framing));
      return waitFor(id);
    },
    notify(method, params) {
      child.stdin.write(encodeFrame({ jsonrpc: '2.0', method, params }, framing));
    },
  };

  function waitFor(id) {
    if (received.has(id)) {
      const message = received.get(id);
      received.delete(id);
      return Promise.resolve(message);
    }
    return new Promise((resolve, reject) => {
      // TIMER-JUSTIFICATION: MCP smoke uses a bounded protocol response timeout to fail hung child servers deterministically.
      const timeout = setTimeout(() => {
        waiters.delete(id);
        reject(new Error(`Timed out waiting for MCP response ${id}`));
      }, 30000);
      waiters.set(id, {
        resolve(message) {
          clearTimeout(timeout);
          resolve(message);
        },
      });
    });
  }

  function readFrame() {
    if (framing === 'ndjson') {
      const lineEnd = output.indexOf('\n');
      if (lineEnd === -1) return null;
      const body = output.slice(0, lineEnd).toString('utf8').replace(/\r$/u, '');
      output = output.slice(lineEnd + 1);
      return body;
    }

    const crlfHeaderEnd = output.indexOf('\r\n\r\n');
    const lfHeaderEnd = output.indexOf('\n\n');
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
    const header = output.slice(0, headerEnd).toString('utf8');
    const lengthMatch = /content-length:\s*(\d+)/iu.exec(header);
    if (!lengthMatch) throw new Error(`Missing Content-Length in MCP frame: ${header}`);
    const contentLength = Number(lengthMatch[1]);
    const start = headerEnd + separatorLength;
    const end = start + contentLength;
    if (output.length < end) return null;
    const body = output.slice(start, end).toString('utf8');
    output = output.slice(end);
    return body;
  }
}

function encodeFrame(message, framing) {
  const body = JSON.stringify(message);
  if (framing === 'ndjson') return `${body}\n`;
  return `Content-Length: ${Buffer.byteLength(body, 'utf8')}\r\n\r\n${body}`;
}
