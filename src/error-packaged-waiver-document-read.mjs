/** Failure to read a packaged-waiver document from its filesystem boundary. */
export class PackagedWaiverDocumentReadError extends Error {
  constructor(registryPath, cause) {
    super(`Cannot load packaged waiver registry ${registryPath}: ${cause.message}`, { cause });
    this.name = "PackagedWaiverDocumentReadError";
    this.registryPath = registryPath;
  }
}
