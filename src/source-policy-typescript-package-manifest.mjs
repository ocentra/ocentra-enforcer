import { scanPackageManifestDependencies } from './source-policy-typescript-package-manifest-dependencies.mjs';
import { scanPackageManifestLockfiles } from './source-policy-typescript-package-manifest-lockfiles.mjs';

export function scanPackageManifestForTypeScriptPolicy(root, filePath) {
  return [
    ...scanPackageManifestLockfiles(root, filePath),
    ...scanPackageManifestDependencies(root, filePath),
  ];
}
