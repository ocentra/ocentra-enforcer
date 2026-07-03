import {
  scanPackageManifestForTypeScriptPolicy as scanPackageManifestForTypeScriptPolicyImpl,
  scanTsConfigStrictness as scanTsConfigStrictnessImpl,
} from './source-policy-typescript-manifest-rules.mjs';

export function scanTsConfigStrictness(root, filePath) {
  return scanTsConfigStrictnessImpl(root, filePath);
}

export function scanPackageManifestForTypeScriptPolicy(root, filePath) {
  return scanPackageManifestForTypeScriptPolicyImpl(root, filePath);
}
