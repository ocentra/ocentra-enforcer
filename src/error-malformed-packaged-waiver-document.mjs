/** Failure to decode malformed JSON from a packaged-waiver document. */
export class MalformedPackagedWaiverDocumentError extends Error {
  constructor(registryPath, cause) {
    super(`Cannot load packaged waiver registry ${registryPath}: ${cause.message}`, { cause });
    this.name = "MalformedPackagedWaiverDocumentError";
    this.registryPath = registryPath;
  }
}
