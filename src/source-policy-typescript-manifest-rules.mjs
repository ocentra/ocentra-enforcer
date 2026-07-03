import {
  scanTsConfigStrictness as scanTsConfigStrictnessImpl,
} from './source-policy-typescript-manifest-tsconfig.mjs';
import {
  scanPackageManifestForTypeScriptPolicy as scanPackageManifestForTypeScriptPolicyImpl,
} from './source-policy-typescript-package-manifest.mjs';

export function scanTsConfigStrictness(root, filePath) {
  return scanTsConfigStrictnessImpl(root, filePath);
}

export function scanPackageManifestForTypeScriptPolicy(root, filePath) {
  return scanPackageManifestForTypeScriptPolicyImpl(root, filePath);
}
