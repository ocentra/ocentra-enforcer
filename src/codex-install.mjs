import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

export const DEFAULT_CODEX_MCP_SERVER_NAME = 'ocentra-enforcer';

export function defaultCodexConfigPath(env = process.env) {
  const codexHome = env.CODEX_HOME || path.join(env.USERPROFILE || env.HOME || os.homedir(), '.codex');
  return path.join(codexHome, 'config.toml');
}

export function createCodexMcpInstallReport({
  packRoot,
  codexConfigPath = defaultCodexConfigPath(),
  serverName = DEFAULT_CODEX_MCP_SERVER_NAME,
  dryRun = false,
  now = new Date(),
}) {
  const resolvedPackRoot = path.resolve(packRoot);
  const resolvedConfigPath = path.resolve(codexConfigPath);
  const serverPath = path.join(resolvedPackRoot, 'mcp', 'rust-rules-mcp.mjs');
  const previousContent = fs.existsSync(resolvedConfigPath) ? fs.readFileSync(resolvedConfigPath, 'utf8') : '';
  const block = codexMcpTomlBlock({ packRoot: resolvedPackRoot, serverName });
  const nextContent = upsertTomlTable(previousContent, `mcp_servers.${serverName}`, block);
  const changed = normalizeNewlines(previousContent) !== normalizeNewlines(nextContent);
  const backupPath =
    changed && fs.existsSync(resolvedConfigPath)
      ? `${resolvedConfigPath}.pre-${serverName}-mcp-${formatTimestamp(now)}.bak`
      : null;

  return {
    ok: fs.existsSync(serverPath),
    command: 'codex-mcp-install',
    serverName,
    packRoot: resolvedPackRoot,
    serverPath,
    codexConfigPath: resolvedConfigPath,
    dryRun: Boolean(dryRun),
    changed,
    backupPath,
    block,
    content: nextContent,
    checks: [
      {
        name: 'mcp server file',
        ok: fs.existsSync(serverPath),
        detail: toTomlPath(serverPath),
      },
      {
        name: 'codex config path',
        ok: resolvedConfigPath.endsWith('config.toml'),
        detail: resolvedConfigPath,
      },
    ],
  };
}

export function applyCodexMcpInstallReport(report) {
  if (!report.ok) {
    const failed = report.checks.find((check) => !check.ok);
    throw new Error(failed ? `${failed.name}: ${failed.detail}` : 'Codex MCP install report is not ok');
  }
  if (report.dryRun || !report.changed) return { ...report, applied: false };

  fs.mkdirSync(path.dirname(report.codexConfigPath), { recursive: true });
  if (report.backupPath) fs.copyFileSync(report.codexConfigPath, report.backupPath);
  fs.writeFileSync(report.codexConfigPath, report.content, 'utf8');
  return { ...report, applied: true };
}

export function createCodexDoctorReport({
  packRoot,
  root = null,
  codexConfigPath = defaultCodexConfigPath(),
  serverName = DEFAULT_CODEX_MCP_SERVER_NAME,
}) {
  const resolvedPackRoot = path.resolve(packRoot);
  const resolvedRoot = root ? path.resolve(root) : null;
  const resolvedConfigPath = path.resolve(codexConfigPath);
  const serverPath = path.join(resolvedPackRoot, 'mcp', 'rust-rules-mcp.mjs');
  const configText = fs.existsSync(resolvedConfigPath) ? fs.readFileSync(resolvedConfigPath, 'utf8') : '';
  const sectionHeader = `[mcp_servers.${serverName}]`;
  const section = extractTomlSection(configText, sectionHeader);
  const nodeVersion = spawnSync('node', ['--version'], { encoding: 'utf8', shell: false });
  const checks = [];

  addCheck(checks, 'node executable', nodeVersion.status === 0, (nodeVersion.stdout || nodeVersion.stderr || 'node not found').trim());
  addCheck(checks, 'pack root', fs.existsSync(resolvedPackRoot), resolvedPackRoot);
  addCheck(checks, 'effect dependency', fs.existsSync(path.join(resolvedPackRoot, 'node_modules', 'effect')), 'node_modules/effect');
  addCheck(checks, 'mcp server file', fs.existsSync(serverPath), toTomlPath(serverPath));
  addCheck(checks, 'codex config file', fs.existsSync(resolvedConfigPath), resolvedConfigPath);
  addCheck(checks, 'codex mcp section', section !== null, sectionHeader);
  addCheck(checks, 'codex mcp command', section?.includes('command = "node"') === true, 'command = "node"');
  addCheck(checks, 'codex mcp args', section?.includes(toTomlPath(serverPath)) === true, toTomlPath(serverPath));
  addCheck(checks, 'codex mcp cwd', section?.includes(toTomlPath(resolvedPackRoot)) === true, toTomlPath(resolvedPackRoot));
  addCheck(checks, 'codex mcp enabled', section?.includes('enabled = true') === true, 'enabled = true');

  if (resolvedRoot) {
    const targetMcpPath = path.join(resolvedRoot, '.mcp.json');
    const targetSkillPath = path.join(resolvedRoot, '.codex', 'skills', 'ocentra-enforcer', 'SKILL.md');
    const targetConfigPath = path.join(resolvedRoot, 'ocentra-enforcer.config.json');
    const targetMcpText = fs.existsSync(targetMcpPath) ? fs.readFileSync(targetMcpPath, 'utf8') : '';
    addCheck(checks, 'target root', fs.existsSync(resolvedRoot), resolvedRoot);
    addCheck(checks, 'target enforcer config', fs.existsSync(targetConfigPath), targetConfigPath, 'warning');
    addCheck(checks, 'target .mcp.json', fs.existsSync(targetMcpPath), targetMcpPath, 'warning');
    addCheck(
      checks,
      'target .mcp.json server path',
      targetMcpReferencesServer(targetMcpText, resolvedRoot, serverName, serverPath),
      toTomlPath(serverPath),
      'warning'
    );
    addCheck(checks, 'target codex skill', fs.existsSync(targetSkillPath), targetSkillPath, 'warning');
  }

  return {
    ok: checks.every((check) => check.ok || check.severity === 'warning'),
    command: 'codex-doctor',
    serverName,
    packRoot: resolvedPackRoot,
    root: resolvedRoot,
    serverPath,
    codexConfigPath: resolvedConfigPath,
    restartRequired: true,
    checks,
    nextSteps: [
      'Restart Codex Desktop or start a new Codex thread after config changes.',
      'If the app still does not show the server, toggle the MCP server in Settings or inspect the Codex config path above.',
      'Run scripts/mcp-smoke.mjs to separate MCP server protocol failures from Codex app config failures.',
    ],
  };
}

export function codexMcpTomlBlock({ packRoot, serverName = DEFAULT_CODEX_MCP_SERVER_NAME }) {
  const serverPath = path.join(path.resolve(packRoot), 'mcp', 'rust-rules-mcp.mjs');
  return [
    `[mcp_servers.${serverName}]`,
    'command = "node"',
    `args = ["${escapeTomlString(toTomlPath(serverPath))}"]`,
    'startup_timeout_sec = 20',
    `cwd = "${escapeTomlString(toTomlPath(path.resolve(packRoot)))}"`,
    'enabled = true',
    '',
  ].join('\n');
}

export function upsertTomlTable(text, tableName, block) {
  const header = `[${tableName}]`;
  const normalized = normalizeNewlines(text);
  const lines = normalized.length === 0 ? [] : normalized.split('\n');
  const start = lines.findIndex((line) => line.trim() === header);
  const blockLines = block.trimEnd().split('\n');

  if (start === -1) {
    const prefix = normalized.trimEnd();
    return `${prefix ? `${prefix}\n\n` : ''}${block.trimEnd()}\n`;
  }

  let end = start + 1;
  while (end < lines.length && !lines[end].trimStart().startsWith('[')) end += 1;

  const replacement = end < lines.length ? [...blockLines, ''] : blockLines;
  const nextLines = [...lines.slice(0, start), ...replacement, ...lines.slice(end)];
  return `${nextLines.join('\n').replace(/\n*$/u, '')}\n`;
}

function extractTomlSection(text, header) {
  const normalized = normalizeNewlines(text);
  const lines = normalized.length === 0 ? [] : normalized.split('\n');
  const start = lines.findIndex((line) => line.trim() === header);
  if (start === -1) return null;
  let end = start + 1;
  while (end < lines.length && !lines[end].trimStart().startsWith('[')) end += 1;
  return lines.slice(start, end).join('\n');
}

function addCheck(checks, name, ok, detail, severity = 'error') {
  checks.push({
    name,
    ok: Boolean(ok),
    severity,
    detail: String(detail ?? ''),
  });
}

function targetMcpReferencesServer(text, root, serverName, serverPath) {
  if (text.includes(toTomlPath(serverPath))) return true;
  try {
    const parsed = JSON.parse(text);
    const args = parsed?.mcpServers?.[serverName]?.args;
    if (!Array.isArray(args)) return false;
    return args.some((arg) => {
      if (typeof arg !== 'string') return false;
      const resolvedArg = path.isAbsolute(arg) ? path.resolve(arg) : path.resolve(root, arg);
      return path.resolve(resolvedArg) === path.resolve(serverPath);
    });
  } catch {
    return false;
  }
}

function normalizeNewlines(value) {
  return String(value ?? '').replace(/\r\n/gu, '\n').replace(/\r/gu, '\n');
}

function toTomlPath(value) {
  return path.resolve(value).replaceAll('\\', '/');
}

function escapeTomlString(value) {
  return String(value).replaceAll('\\', '\\\\').replaceAll('"', '\\"');
}

function formatTimestamp(date) {
  return date
    .toISOString()
    .replace(/[-:]/gu, '')
    .replace(/\.\d{3}Z$/u, 'Z');
}
