import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

const repoRoot = process.cwd();

function readRepoFile(path) {
  return readFileSync(join(repoRoot, path), 'utf8');
}

test('parent desktop package proof separates built portal shell from Rust service runtime', () => {
  const tauriConfig = readRepoFile('apps/parent-desktop/src-tauri/tauri.conf.json');
  const tauriLib = readRepoFile('apps/parent-desktop/src-tauri/src/lib.rs');
  const packageJson = readRepoFile('apps/parent-desktop/package.json');
  const devDesktopScript = readRepoFile('scripts/dev/dev-parent-desktop.mjs');
  const protocolValues = readRepoFile('crates/agent-protocol/src/constants/value.rs');
  const windowsServiceConfig = readRepoFile('scripts/release/windows/OcentraParentAgentService.xml');
  const windowsInstaller = readRepoFile('scripts/release/windows/OcentraParentAgent.wxs');
  const lifecycleHost = readRepoFile('scripts/release/windows/package-lifecycle-host.mjs');

  assert.match(tauriConfig, /"frontendDist": "\.\.\/\.\.\/portal\/dist"/u);
  assert.match(tauriConfig, /"devUrl": "http:\/\/127\.0\.0\.1:4478"/u);
  assert.match(tauriConfig, /ws:\/\/127\.0\.0\.1:4478/u);
  assert.doesNotMatch(tauriConfig, /ws:\/\/127\.0\.0\.1:4477/u);
  assert.match(
    packageJson,
    /"portal:build": "npm --prefix \.\.\/\.\. run build:contracts && npm --prefix \.\.\/\.\. run build --workspace @ocentra-parent\/portal"/u
  );
  assert.match(devDesktopScript, /Desktop product host bridge uses Tauri invoke\/listen; local dev bridge stays web-only\./u);
  assert.match(devDesktopScript, /ParentDevEnv\.AgentAddress/u);
  assert.match(devDesktopScript, /ParentDevEnv\.PortalAgentWebSocketUrl/u);
  assert.ok(devDesktopScript.includes("cargo', ['tauri', 'dev', '-c', generatedConfigPath]"));

  assert.match(tauriLib, /service_health_endpoint/u);
  assert.match(tauriLib, /runtime_readiness_state/u);
  assert.match(tauriLib, /package_frontend_state/u);
  assert.match(tauriLib, /hmr_backend_state/u);
  assert.match(tauriLib, /process_ownership_state/u);
  assert.match(tauriLib, /controller_route_state/u);
  assert.match(tauriLib, /observer_read_only_state/u);
  assert.match(tauriLib, /source_custody_state/u);
  assert.match(tauriLib, /relay_route_state/u);
  assert.match(tauriLib, /parent_cache_state/u);
  assert.match(tauriLib, /parent_storage_state/u);
  assert.match(tauriLib, /service_launch_owner_state/u);
  assert.match(tauriLib, /service_launch_strategy_state/u);
  assert.match(tauriLib, /service_connect_timeout_ms/u);
  assert.match(tauriLib, /package_service_manager_state/u);
  assert.match(tauriLib, /package_health_probe_state/u);
  assert.match(tauriLib, /port_ownership_state/u);
  assert.match(tauriLib, /port_conflict_policy_state/u);
  assert.match(tauriLib, /blank_window_regression_state/u);
  assert.match(tauriLib, /package_preview_state/u);
  assert.match(tauriLib, /update_channel_state/u);
  assert.match(tauriLib, /rollback_state/u);
  assert.match(tauriLib, /signing_state/u);
  assert.match(tauriLib, /notarization_state/u);
  assert.match(tauriLib, /store_distribution_state/u);
  assert.match(tauriLib, /support_diagnostics_state/u);
  assert.match(tauriLib, /support_redaction_state/u);
  assert.match(tauriLib, /platform_matrix_state/u);
  assert.match(tauriLib, /release_branch_state/u);
  assert.match(tauriLib, /artifact_proof_state/u);
  assert.match(tauriLib, /TcpStream::connect_timeout/u);
  assert.match(tauriLib, /PARENT_DESKTOP_RUNTIME_READY/u);
  assert.match(tauriLib, /PARENT_DESKTOP_RUNTIME_DEGRADED/u);
  assert.doesNotMatch(tauriLib, /VITE_|devUrl|portal:dev|vite_backend_state/u);

  assert.match(protocolValues, /PARENT_DESKTOP_FRONTEND_BUILT_PORTAL_DIST/u);
  assert.match(protocolValues, /PARENT_DESKTOP_HMR_BACKEND_NOT_USED/u);
  assert.match(protocolValues, /PARENT_DESKTOP_PROCESS_OWNER_SHELL_ONLY/u);
  assert.match(protocolValues, /PARENT_DESKTOP_CONTROLLER_ROUTE_ACTIVE_CONTROLLER/u);
  assert.match(protocolValues, /PARENT_DESKTOP_OBSERVER_READ_ONLY/u);
  assert.match(protocolValues, /PARENT_DESKTOP_SOURCE_CUSTODY_LIVE_LOCAL_NETWORK/u);
  assert.match(protocolValues, /PARENT_DESKTOP_RELAY_ROUTE_UNAVAILABLE/u);
  assert.match(protocolValues, /PARENT_DESKTOP_PARENT_CACHE_UNAVAILABLE/u);
  assert.match(protocolValues, /PARENT_DESKTOP_PARENT_STORAGE_UNAVAILABLE/u);
  assert.match(protocolValues, /PARENT_DESKTOP_SERVICE_LAUNCH_OWNER_PACKAGE_SERVICE/u);
  assert.match(protocolValues, /PARENT_DESKTOP_SERVICE_LAUNCH_STRATEGY_CONNECT_OR_DEGRADE/u);
  assert.match(protocolValues, /PARENT_DESKTOP_PACKAGE_SERVICE_AUTO_START/u);
  assert.match(protocolValues, /PARENT_DESKTOP_PACKAGE_HEALTH_PROBE_REQUIRED/u);
  assert.match(protocolValues, /PARENT_DESKTOP_PORT_OWNERSHIP_FIXED_LOOPBACK/u);
  assert.match(protocolValues, /PARENT_DESKTOP_PORT_CONFLICT_POLICY_NO_FOREIGN_RECLAIM/u);
  assert.match(protocolValues, /PARENT_DESKTOP_BLANK_WINDOW_GUARD_FRONTEND_DIST/u);
  assert.match(protocolValues, /PARENT_DESKTOP_PACKAGE_PREVIEW_UNSIGNED/u);
  assert.match(protocolValues, /PARENT_DESKTOP_UPDATE_CHANNEL_SCAFFOLD/u);
  assert.match(protocolValues, /PARENT_DESKTOP_ROLLBACK_UNAVAILABLE/u);
  assert.match(protocolValues, /PARENT_DESKTOP_SIGNING_MANUAL_REQUIRED/u);
  assert.match(protocolValues, /PARENT_DESKTOP_NOTARIZATION_MANUAL_REQUIRED/u);
  assert.match(protocolValues, /PARENT_DESKTOP_STORE_DISTRIBUTION_MANUAL_REQUIRED/u);
  assert.match(protocolValues, /PARENT_DESKTOP_SUPPORT_DIAGNOSTICS_REDACTED/u);
  assert.match(protocolValues, /PARENT_DESKTOP_SUPPORT_OUTPUT_ALLOWED_FIELDS/u);
  assert.match(protocolValues, /PARENT_DESKTOP_PLATFORM_MATRIX_SPLIT_PROOF_ROWS/u);
  assert.match(protocolValues, /PARENT_DESKTOP_RELEASE_BRANCH_PRODUCTION_PROMOTION_REQUIRED/u);
  assert.match(protocolValues, /PARENT_DESKTOP_ARTIFACT_PROOF_CI_PREVIEW/u);

  assert.match(windowsServiceConfig, /<executable>%BASE%\\ocentra-parent-agent-service\.exe<\/executable>/u);
  assert.match(windowsServiceConfig, /<env name="OCENTRA_PARENT_AGENT_ADDR" value="127\.0\.0\.1:4477" \/>/u);
  assert.match(windowsServiceConfig, /<startmode>Automatic<\/startmode>/u);
  assert.match(windowsInstaller, /<ServiceControl[\s\S]*Name="OcentraParentAgent"[\s\S]*Start="install"/u);
  assert.match(lifecycleHost, /DEFAULT_HEALTH_URL = 'http:\/\/127\.0\.0\.1:4477\/health'/u);
  assert.match(lifecycleHost, /ensureServicesRunning\(\)/u);
  assert.match(lifecycleHost, /service-health-unavailable/u);
});
