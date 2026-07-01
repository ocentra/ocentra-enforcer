import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

const repoRoot = process.cwd();

function readRepoFile(path) {
  return readFileSync(join(repoRoot, path), 'utf8');
}

test('parent Android mobile app is a separate buildable product', () => {
  assert.equal(existsSync(join(repoRoot, 'platforms/android/parent/settings.gradle')), true);
  assert.equal(existsSync(join(repoRoot, 'platforms/android/parent/gradlew')), true);
  assert.equal(existsSync(join(repoRoot, 'platforms/android/parent/gradle/wrapper/gradle-wrapper.jar')), true);

  const manifest = readRepoFile('platforms/android/parent/app/src/main/AndroidManifest.xml');
  const buildFile = readRepoFile('platforms/android/parent/app/build.gradle');
  const activity = readRepoFile(
    'platforms/android/parent/app/src/main/java/ca/ocentra/parent/mobile/MainActivity.java'
  );
  const strings = readRepoFile('platforms/android/parent/app/src/main/res/values/strings.xml');

  assert.match(buildFile, /namespace = 'ca\.ocentra\.parent\.mobile'/u);
  assert.match(buildFile, /applicationId = 'ca\.ocentra\.parent\.mobile'/u);
  assert.match(manifest, /android\.intent\.action\.MAIN/u);
  assert.match(manifest, /android\.intent\.category\.LAUNCHER/u);
  assert.match(activity, /package ca\.ocentra\.parent\.mobile;/u);
  assert.match(strings, /Ocentra Parent Mobile Android scaffold/u);
  assert.doesNotMatch(manifest, /OcentraParentAgentService/u);
});

test('parent iOS mobile app is a separate simulator product', () => {
  assert.equal(existsSync(join(repoRoot, 'platforms/ios/OcentraParentMobile.xcodeproj/project.pbxproj')), true);

  const project = readRepoFile('platforms/ios/OcentraParentMobile.xcodeproj/project.pbxproj');
  const scheme = readRepoFile(
    'platforms/ios/OcentraParentMobile.xcodeproj/xcshareddata/xcschemes/OcentraParentMobile.xcscheme'
  );
  const statusView = readRepoFile('platforms/ios/OcentraParentMobile/ParentMobileStatusViewController.swift');
  const plist = readRepoFile('platforms/ios/OcentraParentMobile/Info.plist');

  assert.match(project, /PRODUCT_BUNDLE_IDENTIFIER = ca\.ocentra\.parent\.mobile;/u);
  assert.match(project, /PRODUCT_NAME = OcentraParentMobile;/u);
  assert.match(scheme, /OcentraParentMobile\.app/u);
  assert.match(statusView, /Ocentra Parent Mobile iOS scaffold/u);
  assert.match(statusView, /child-agent-parity=not-claimed/u);
  assert.match(plist, /CFBundleIdentifier/u);
});

test('parent mobile release scripts and smoke inputs are separate from child agent package scripts', () => {
  const androidRelease = readRepoFile('scripts/release/parent-android/build-parent-mobile-package.mjs');
  const iosRelease = readRepoFile('scripts/release/parent-ios/build-parent-mobile-simulator-app.sh');
  const androidSmoke = readRepoFile('scripts/smoke/android-apk-smoke.sh');
  const iosSmoke = readRepoFile('scripts/smoke/ios-simulator-smoke.sh');

  assert.match(androidRelease, /platforms',\s*'android',\s*'parent'/u);
  assert.match(androidRelease, /ocentra-parent-mobile-android-debug-latest\.apk/u);
  assert.match(iosRelease, /OcentraParentMobile\.xcodeproj/u);
  assert.match(iosRelease, /ocentra-parent-mobile-ios-simulator-latest\.zip/u);
  assert.match(androidSmoke, /\$\{2:-ca\.ocentra\.parent\.agent\}/u);
  assert.match(iosSmoke, /\$\{2:-ca\.ocentra\.parent\.agent\}/u);
});
