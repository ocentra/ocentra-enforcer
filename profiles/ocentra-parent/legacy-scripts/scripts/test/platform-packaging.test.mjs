import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

const repoRoot = process.cwd();

function readRepoFile(path) {
  return readFileSync(join(repoRoot, path), 'utf8');
}

test('production release workflow publishes only from production branch', () => {
  const workflow = readRepoFile('.github/workflows/release.yml');

  assert.match(workflow, /branches:\s+- production/u);
  assert.match(workflow, /release-decision:/u);
  assert.match(workflow, /node scripts\/release\/decide-production-release\.mjs/u);
  assert.match(workflow, /if: needs\.release-decision\.outputs\.release-required == 'true'/u);
  assert.match(workflow, /Build signed Windows MSI package/u);
  assert.match(workflow, /Check production release secrets/u);
  assert.match(workflow, /OCENTRA_PARENT_UPDATE_SIGNING_KEY_BASE64/u);
  assert.match(workflow, /scripts\/smoke\/windows-msi-smoke\.ps1/u);
});

test('package preview workflow builds every scaffolded platform', () => {
  const workflow = readRepoFile('.github/workflows/package-preview.yml');

  for (const jobName of ['windows-msi', 'linux-deb', 'macos-pkg', 'android-apk', 'ios-simulator']) {
    assert.match(workflow, new RegExp(`${jobName}:`, 'u'));
  }
  assert.match(workflow, /OCENTRA_PARENT_ALLOW_EPHEMERAL_UPDATE_KEY: 'true'/u);
  assert.match(workflow, /scripts\/release\/linux\/build-agent-package\.sh/u);
  assert.match(workflow, /scripts\/release\/macos\/build-agent-package\.sh/u);
  assert.match(workflow, /scripts\/release\/android\/build-agent-package\.mjs/u);
  assert.match(workflow, /scripts\/release\/ios\/build-simulator-app\.sh/u);
  assert.match(workflow, /scripts\/smoke\/windows-msi-smoke\.ps1/u);
  assert.match(workflow, /scripts\/smoke\/linux-deb-smoke\.sh/u);
  assert.match(workflow, /scripts\/smoke\/macos-pkg-smoke\.sh/u);
  assert.match(workflow, /scripts\/smoke\/android-apk-smoke\.sh/u);
  assert.match(workflow, /scripts\/smoke\/ios-simulator-smoke\.sh/u);
  assert.match(workflow, /reactivecircus\/android-emulator-runner@v2/u);
  assert.match(workflow, /Enable KVM for Android emulator/u);
  assert.match(workflow, /emulator-boot-timeout: 900/u);
  assert.match(workflow, /Upload Windows MSI smoke logs/u);
});

test('package smoke scripts check real uninstall and emit diagnostics', () => {
  const linuxSmoke = readRepoFile('scripts/smoke/linux-deb-smoke.sh');
  const windowsSmoke = readRepoFile('scripts/smoke/windows-msi-smoke.ps1');

  assert.match(linuxSmoke, /\$\{db:Status-Abbrev\}/u);
  assert.match(linuxSmoke, /Agent executable remained after remove/u);
  assert.match(windowsSmoke, /windows-msi-install\.log/u);
  assert.match(windowsSmoke, /\/L\*v/u);
});

test('parent desktop Tauri package connects to the Rust service instead of Vite backend', () => {
  const cargoToml = readRepoFile('apps/parent-desktop/src-tauri/Cargo.toml');
  const tauriConfig = readRepoFile('apps/parent-desktop/src-tauri/tauri.conf.json');
  const tauriLib = readRepoFile('apps/parent-desktop/src-tauri/src/lib.rs');
  const packageJson = readRepoFile('apps/parent-desktop/package.json');

  assert.match(cargoToml, /ocentra-parent-agent-protocol/u);
  assert.match(tauriConfig, /"frontendDist": "\.\.\/\.\.\/portal\/dist"/u);
  assert.match(tauriConfig, /ws:\/\/127\.0\.0\.1:4478/u);
  assert.doesNotMatch(tauriConfig, /ws:\/\/127\.0\.0\.1:4477/u);
  assert.match(tauriLib, /parent_platform_proof_state/u);
  assert.match(tauriLib, /TcpStream::connect_timeout/u);
  assert.match(tauriLib, /activity_adapter_state/u);
  assert.match(tauriLib, /parent_assistant_provider_state/u);
  assert.match(tauriLib, /runtime_readiness_state/u);
  assert.match(tauriLib, /service_health_endpoint/u);
  assert.match(tauriLib, /route_source_state/u);
  assert.match(tauriLib, /degraded_source_state/u);
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
  assert.match(tauriLib, /PARENT_DESKTOP_BACKEND_RUST_SERVICE/u);
  assert.match(tauriLib, /PARENT_DESKTOP_RUNTIME_READY/u);
  assert.match(tauriLib, /PARENT_DESKTOP_RUNTIME_DEGRADED/u);
  assert.doesNotMatch(tauriLib, /VITE_|devUrl|portal:dev|vite_backend_state/u);
  assert.match(packageJson, /"tauri:check": "cargo check --manifest-path src-tauri\/Cargo.toml"/u);
});

test('dependency policy workflow audits dependencies and writes SBOM metadata', () => {
  const workflow = readRepoFile('.github/workflows/dependency-policy.yml');
  const packageJson = readRepoFile('package.json');

  assert.match(workflow, /cargo install cargo-audit --locked/u);
  assert.match(workflow, /npm run security:deps/u);
  assert.match(workflow, /npm run security:sbom/u);
  assert.match(workflow, /target\/security\/\*\.json/u);
  assert.match(readRepoFile('scripts/security/write-sbom.mjs'), /--sbom-format=cyclonedx/u);
  assert.match(packageJson, /"security:deps": "node scripts\/security\/check-dependency-policy\.mjs"/u);
  assert.match(packageJson, /"security:sbom": "node scripts\/security\/write-sbom\.mjs"/u);
});

test('toolchains are pinned for Rust and Android packaging', () => {
  const rustToolchain = readRepoFile('rust-toolchain.toml');
  const setupCi = readRepoFile('.github/actions/setup-ci/action.yml');
  const androidBuilder = readRepoFile('scripts/release/android/build-agent-package.mjs');
  const gradleWrapper = readRepoFile('platforms/android/agent/gradle/wrapper/gradle-wrapper.properties');

  assert.match(rustToolchain, /channel = "1\.90\.0"/u);
  assert.match(setupCi, /rust-toolchain\.toml/u);
  assert.match(androidBuilder, /gradlew\.bat assembleDebug/u);
  assert.match(androidBuilder, /\.\/gradlew/u);
  assert.match(gradleWrapper, /gradle-8\.12\.1-bin\.zip/u);
});

test('Linux and macOS packages install real service managers', () => {
  const linuxUnit = readRepoFile('scripts/release/linux/ocentra-parent-agent.service');
  const macLaunchd = readRepoFile('scripts/release/macos/ca.ocentra.parent.agent.plist');

  assert.match(linuxUnit, /ExecStart=\/opt\/ocentra\/ocentra-parent-agent\/bin\/ocentra-parent-agent-service/u);
  assert.match(linuxUnit, /WantedBy=multi-user\.target/u);
  assert.match(macLaunchd, /ca\.ocentra\.parent\.agent/u);
  assert.match(macLaunchd, /\/Library\/Ocentra\/Ocentra Parent Agent\/bin\/ocentra-parent-agent-service/u);
});

test('mobile platform projects define real installable app targets', () => {
  const androidManifest = readRepoFile('platforms/android/parent/app/src/main/AndroidManifest.xml');
  const iosProject = readRepoFile('platforms/ios/OcentraParentMobile.xcodeproj/project.pbxproj');
  const parentMobileSourceProof = readRepoFile('scripts/test/parent-mobile-package-source-artifact-proof.mjs');

  assert.match(androidManifest, /android\.intent\.action\.MAIN/u);
  assert.match(androidManifest, /android\.intent\.category\.LAUNCHER/u);
  assert.doesNotMatch(androidManifest, /OcentraParentAgentService/u);
  assert.match(iosProject, /productType = "com\.apple\.product-type\.application"/u);
  assert.match(iosProject, /PRODUCT_BUNDLE_IDENTIFIER = ca\.ocentra\.parent\.mobile/u);
  assert.match(parentMobileSourceProof, /child-agent-parity=not-claimed/u);
  assert.match(parentMobileSourceProof, /parent mobile release scripts and smoke inputs are separate from child agent package scripts/u);
});
