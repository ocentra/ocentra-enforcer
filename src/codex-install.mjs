import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

// PUBLIC-API-BUDGET-JUSTIFICATION: Codex install exposes a stable adapter surface consumed by CLI, MCP, and tests.
export const DEFAULT_CODEX_MCP_SERVER_NAME = 'ocentra-enforcer';
export const DEFAULT_CODEX_SKILL_NAME = 'ocentra-enforcer';
const GLOBAL_AGENTS_START = '<!-- ocentra-enforcer:start -->';
const GLOBAL_AGENTS_END = '<!-- ocentra-enforcer:end -->';

export function defaultCodexConfigPath(env = process.env) {
  const codexHome = env.CODEX_HOME || path.join(env.USERPROFILE || env.HOME || os.homedir(), '.codex');
  return path.join(codexHome, 'config.toml');
}

export function createCodexMcpInstallReport({
  packRoot,
  codexConfigPath = defaultCodexConfigPath(),
  serverName = DEFAULT_CODEX_MCP_SERVER_NAME,
  ledgerRoot = null,
  installSkill = true,
  installGlobalAgents = true,
  dryRun = false,
  now = new Date(),
}) {
  const resolvedPackRoot = path.resolve(packRoot);
  const resolvedConfigPath = path.resolve(codexConfigPath);
  const resolvedLedgerRoot = path.resolve(ledgerRoot ?? path.join(resolvedPackRoot, '.ledger'));
  const serverPath = path.join(resolvedPackRoot, 'mcp', 'ocentra-enforcer-mcp.mjs');
  const previousContent = fs.existsSync(resolvedConfigPath) ? fs.readFileSync(resolvedConfigPath, 'utf8') : '';
  const block = codexMcpTomlBlock({ packRoot: resolvedPackRoot, serverName, ledgerRoot: resolvedLedgerRoot });
  const nextContent = upsertTomlTable(previousContent, `mcp_servers.${serverName}`, block);
  const codexHome = path.dirname(resolvedConfigPath);
  const skillSource = path.join(resolvedPackRoot, 'skills', DEFAULT_CODEX_SKILL_NAME);
  const skillTarget = path.join(codexHome, 'skills', DEFAULT_CODEX_SKILL_NAME);
  const globalAgentsPath = path.join(codexHome, 'AGENTS.md');
  const previousGlobalAgents = fs.existsSync(globalAgentsPath) ? fs.readFileSync(globalAgentsPath, 'utf8') : '';
  const globalAgentsBlock = globalAgentsInstructionBlock({ packRoot: resolvedPackRoot, serverName });
  const nextGlobalAgents = upsertManagedBlock(previousGlobalAgents, GLOBAL_AGENTS_START, GLOBAL_AGENTS_END, globalAgentsBlock);
  const changed = normalizeNewlines(previousContent) !== normalizeNewlines(nextContent);
  const globalAgentsChanged = installGlobalAgents && normalizeNewlines(previousGlobalAgents) !== normalizeNewlines(nextGlobalAgents);
  const backupPath =
    changed && fs.existsSync(resolvedConfigPath)
      ? `${resolvedConfigPath}.pre-${serverName}-mcp-${formatTimestamp(now)}.bak`
      : null;
  const globalAgentsBackupPath =
    globalAgentsChanged && fs.existsSync(globalAgentsPath)
      ? `${globalAgentsPath}.pre-${serverName}-${formatTimestamp(now)}.bak`
      : null;

  return {
    ok: fs.existsSync(serverPath),
    command: 'codex-mcp-install',
    serverName,
    packRoot: resolvedPackRoot,
    serverPath,
    ledgerRoot: resolvedLedgerRoot,
    codexConfigPath: resolvedConfigPath,
    dryRun: Boolean(dryRun),
    changed,
    backupPath,
    skillSource,
    skillTarget,
    installSkill: Boolean(installSkill),
    skillChanged:
      Boolean(installSkill) &&
      (!fs.existsSync(path.join(skillTarget, 'SKILL.md')) ||
        fs.readFileSync(path.join(skillTarget, 'SKILL.md'), 'utf8') !== fs.readFileSync(path.join(skillSource, 'SKILL.md'), 'utf8')),
    globalAgentsPath,
    installGlobalAgents: Boolean(installGlobalAgents),
    globalAgentsChanged,
    globalAgentsBackupPath,
    globalAgentsBlock,
    globalAgentsContent: nextGlobalAgents,
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
      {
        name: 'ledger root',
        ok: resolvedLedgerRoot.length > 0,
        detail: resolvedLedgerRoot,
      },
      {
        name: 'canonical skill source',
        ok: fs.existsSync(path.join(skillSource, 'SKILL.md')),
        detail: skillSource,
      },
    ],
  };
}

export function applyCodexMcpInstallReport(report) {
  if (!report.ok) {
    const failed = report.checks.find((check) => !check.ok);
    throw new Error(failed ? `${failed.name}: ${failed.detail}` : 'Codex MCP install report is not ok');
  }
  if (report.dryRun) return { ...report, applied: false };

  fs.mkdirSync(report.ledgerRoot, { recursive: true });
  const ledgerGitignore = path.join(report.ledgerRoot, '.gitignore');
  if (!fs.existsSync(ledgerGitignore)) {
    fs.writeFileSync(ledgerGitignore, '*\n!.gitignore\n', 'utf8');
  }

  if (report.changed) {
    fs.mkdirSync(path.dirname(report.codexConfigPath), { recursive: true });
    if (report.backupPath) fs.copyFileSync(report.codexConfigPath, report.backupPath);
    fs.writeFileSync(report.codexConfigPath, report.content, 'utf8');
  }
  if (report.installSkill && report.skillChanged) {
    copyDir(report.skillSource, report.skillTarget);
  }
  if (report.installGlobalAgents && report.globalAgentsChanged) {
    fs.mkdirSync(path.dirname(report.globalAgentsPath), { recursive: true });
    if (report.globalAgentsBackupPath) fs.copyFileSync(report.globalAgentsPath, report.globalAgentsBackupPath);
    fs.writeFileSync(report.globalAgentsPath, report.globalAgentsContent, 'utf8');
  }
  return { ...report, applied: report.changed || report.skillChanged || report.globalAgentsChanged };
}

export function createCodexUninstallReport({
  packRoot,
  codexConfigPath = defaultCodexConfigPath(),
  serverName = DEFAULT_CODEX_MCP_SERVER_NAME,
  removeSkill = true,
  removeGlobalAgents = true,
  dryRun = false,
  now = new Date(),
}) {
  const resolvedConfigPath = path.resolve(codexConfigPath);
  const resolvedPackRoot = path.resolve(packRoot);
  const codexHome = path.dirname(resolvedConfigPath);
  const previousContent = fs.existsSync(resolvedConfigPath) ? fs.readFileSync(resolvedConfigPath, 'utf8') : '';
  const nextContent = removeTomlTable(previousContent, `mcp_servers.${serverName}`);
  const changed = normalizeNewlines(previousContent) !== normalizeNewlines(nextContent);
  const skillTarget = path.join(codexHome, 'skills', DEFAULT_CODEX_SKILL_NAME);
  const globalAgentsPath = path.join(codexHome, 'AGENTS.md');
  const previousGlobalAgents = fs.existsSync(globalAgentsPath) ? fs.readFileSync(globalAgentsPath, 'utf8') : '';
  const nextGlobalAgents = removeManagedBlock(previousGlobalAgents, GLOBAL_AGENTS_START, GLOBAL_AGENTS_END);
  const globalAgentsChanged = removeGlobalAgents && normalizeNewlines(previousGlobalAgents) !== normalizeNewlines(nextGlobalAgents);
  return {
    ok: true,
    command: 'codex-uninstall',
    serverName,
    packRoot: resolvedPackRoot,
    codexConfigPath: resolvedConfigPath,
    dryRun: Boolean(dryRun),
    changed,
    content: nextContent,
    backupPath:
      changed && fs.existsSync(resolvedConfigPath)
        ? `${resolvedConfigPath}.pre-${serverName}-uninstall-${formatTimestamp(now)}.bak`
        : null,
    removeSkill: Boolean(removeSkill),
    skillTarget,
    skillChanged: Boolean(removeSkill) && fs.existsSync(skillTarget),
    removeGlobalAgents: Boolean(removeGlobalAgents),
    globalAgentsPath,
    globalAgentsChanged,
    globalAgentsContent: nextGlobalAgents,
    globalAgentsBackupPath:
      globalAgentsChanged && fs.existsSync(globalAgentsPath)
        ? `${globalAgentsPath}.pre-${serverName}-uninstall-${formatTimestamp(now)}.bak`
        : null,
  };
}

export function applyCodexUninstallReport(report) {
  if (report.dryRun) return { ...report, applied: false };
  if (report.changed) {
    if (report.backupPath) fs.copyFileSync(report.codexConfigPath, report.backupPath);
    fs.writeFileSync(report.codexConfigPath, report.content, 'utf8');
  }
  if (report.skillChanged) fs.rmSync(report.skillTarget, { recursive: true, force: true });
  if (report.globalAgentsChanged) {
    if (report.globalAgentsBackupPath) fs.copyFileSync(report.globalAgentsPath, report.globalAgentsBackupPath);
    fs.writeFileSync(report.globalAgentsPath, report.globalAgentsContent, 'utf8');
  }
  return { ...report, applied: report.changed || report.skillChanged || report.globalAgentsChanged };
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
  const serverPath = path.join(resolvedPackRoot, 'mcp', 'ocentra-enforcer-mcp.mjs');
  const configText = fs.existsSync(resolvedConfigPath) ? fs.readFileSync(resolvedConfigPath, 'utf8') : '';
  const sectionHeader = `[mcp_servers.${serverName}]`;
  const section = extractTomlSection(configText, sectionHeader);
  const nodeVersion = spawnSync('node', ['--version'], { encoding: 'utf8', shell: false });
  const checks = [];

  addCheck(checks, 'node executable', nodeVersion.status === 0, (nodeVersion.stdout || nodeVersion.stderr || 'node not found').trim());
  addCheck(checks, 'pack root', fs.existsSync(resolvedPackRoot), resolvedPackRoot);
  addCheck(checks, 'effect dependency', fs.existsSync(path.join(resolvedPackRoot, 'node_modules', 'effect')), 'node_modules/effect');
  addCheck(checks, 'mcp server file', fs.existsSync(serverPath), toTomlPath(serverPath));
  const ledgerRoot = extractLedgerRoot(section) ?? path.join(resolvedPackRoot, '.ledger');
  addCheck(checks, 'ledger root configured', section?.includes('OCENTRA_LEDGER_HOME') === true, toTomlPath(ledgerRoot), 'warning');
  addCheck(checks, 'ledger root directory', fs.existsSync(ledgerRoot), toTomlPath(ledgerRoot), 'warning');
  addCheck(checks, 'codex config file', fs.existsSync(resolvedConfigPath), resolvedConfigPath);
  addCheck(checks, 'codex mcp section', section !== null, sectionHeader);
  addCheck(checks, 'codex mcp command', section?.includes('command = "node"') === true, 'command = "node"');
  addCheck(checks, 'codex mcp args', section?.includes(toTomlPath(serverPath)) === true, toTomlPath(serverPath));
  addCheck(checks, 'codex mcp cwd', section?.includes(toTomlPath(resolvedPackRoot)) === true, toTomlPath(resolvedPackRoot));
  addCheck(checks, 'codex mcp enabled', section?.includes('enabled = true') === true, 'enabled = true');
  addCheck(checks, 'user enforcer skill', fs.existsSync(path.join(path.dirname(resolvedConfigPath), 'skills', DEFAULT_CODEX_SKILL_NAME, 'SKILL.md')), DEFAULT_CODEX_SKILL_NAME, 'warning');
  const globalAgentsPath = path.join(path.dirname(resolvedConfigPath), 'AGENTS.md');
  const globalAgentsText = fs.existsSync(globalAgentsPath) ? fs.readFileSync(globalAgentsPath, 'utf8') : '';
  addCheck(checks, 'global AGENTS enforcer block', globalAgentsText.includes(GLOBAL_AGENTS_START), globalAgentsPath, 'warning');

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

export function codexMcpTomlBlock({ packRoot, serverName = DEFAULT_CODEX_MCP_SERVER_NAME, ledgerRoot = null }) {
  const serverPath = path.join(path.resolve(packRoot), 'mcp', 'ocentra-enforcer-mcp.mjs');
  const resolvedLedgerRoot = path.resolve(ledgerRoot ?? path.join(path.resolve(packRoot), '.ledger'));
  return [
    `[mcp_servers.${serverName}]`,
    'command = "node"',
    `args = ["${escapeTomlString(toTomlPath(serverPath))}"]`,
    `env = { OCENTRA_LEDGER_HOME = "${escapeTomlString(toTomlPath(resolvedLedgerRoot))}" }`,
    'startup_timeout_sec = 20',
    `cwd = "${escapeTomlString(toTomlPath(path.resolve(packRoot)))}"`,
    'enabled = true',
    '',
  ].join('\n');
}

export function globalAgentsInstructionBlock({ packRoot, serverName = DEFAULT_CODEX_MCP_SERVER_NAME }) {
  const normalizedPackRoot = toTomlPath(packRoot);
  return [
    GLOBAL_AGENTS_START,
    '# Ocentra Enforcer',
    '',
    'Use Ocentra Enforcer for project-independent enforcement, coordination, and compact diagnostics.',
    `MCP server name: \`${serverName}\`.`,
    `Pack root: \`${normalizedPackRoot}\`.`,
    `Ledger root: \`${toTomlPath(path.join(path.resolve(packRoot), '.ledger'))}\`. Hubs live under this folder, for example \`.ledger/<hub>\`.`,
    '',
    'Before relying on raw terminal output, prefer:',
    '- `ocentra_enforcer_route` for indexed rule routing.',
    '- `ocentra_enforcer_check` / `ocentra_enforcer_scan` for hard validation.',
    '- `ocentra_enforcer_run` plus `ocentra_enforcer_last_failure` for compact harness diagnostics.',
    '- `ocentra_enforcer_coordination_health` / `claim` / `guard` for Codex lane/mail/exact-file coordination.',
    '',
    'Coordination is a Codex/harness concern, not a product-repo concern. Live state belongs under the Enforcer install ledger root by default. Use `--state-root` or `LEDGER_ROOT` only for explicit exact-root repair/import operations.',
    GLOBAL_AGENTS_END,
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

export function removeTomlTable(text, tableName) {
  const header = `[${tableName}]`;
  const normalized = normalizeNewlines(text);
  const lines = normalized.length === 0 ? [] : normalized.split('\n');
  const start = lines.findIndex((line) => line.trim() === header);
  if (start === -1) return normalized;
  let end = start + 1;
  while (end < lines.length && !lines[end].trimStart().startsWith('[')) end += 1;
  const next = [...lines.slice(0, start), ...lines.slice(end)].join('\n').replace(/\n{3,}/gu, '\n\n');
  return `${next.replace(/\n*$/u, '')}${next.trim().length > 0 ? '\n' : ''}`;
}

export function upsertManagedBlock(text, startMarker, endMarker, block) {
  const without = removeManagedBlock(text, startMarker, endMarker).trimEnd();
  return `${without ? `${without}\n\n` : ''}${block.trimEnd()}\n`;
}

export function removeManagedBlock(text, startMarker, endMarker) {
  const normalized = normalizeNewlines(text);
  const start = normalized.indexOf(startMarker);
  const end = normalized.indexOf(endMarker, start === -1 ? 0 : start);
  if (start === -1 || end === -1) return normalized;
  const afterEnd = end + endMarker.length;
  return `${normalized.slice(0, start)}${normalized.slice(afterEnd)}`.replace(/\n{3,}/gu, '\n\n').replace(/^\n+/u, '');
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

function extractLedgerRoot(section) {
  if (!section) return null;
  const match = /OCENTRA_LEDGER_HOME\s*=\s*"([^"]+)"/u.exec(section);
  return match ? path.resolve(match[1]) : null;
}

function addCheck(checks, name, ok, detail, severity = 'error') {
  checks.push({
    name,
    ok: Boolean(ok),
    severity,
    detail: String(detail ?? ''),
  });
}

function copyDir(source, target) {
  fs.rmSync(target, { recursive: true, force: true });
  fs.mkdirSync(target, { recursive: true });
  for (const entry of fs.readdirSync(source, { withFileTypes: true })) {
    const sourcePath = path.join(source, entry.name);
    const targetPath = path.join(target, entry.name);
    if (entry.isDirectory()) copyDir(sourcePath, targetPath);
    else if (entry.isFile()) fs.copyFileSync(sourcePath, targetPath);
  }
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
