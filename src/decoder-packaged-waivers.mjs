//! Decoder boundary for the on-disk packaged-waiver document.
// BOUNDARY-INVARIANT: raw file bytes enter only here and the caller immediately
// validates the document's waiver array before any record is used.
// boundaryOwnerNote: packaged-waiver filesystem JSON transport.
import fs from "node:fs";

/** Decode untrusted packaged-waiver JSON at the filesystem boundary. */
export function decodePackagedWaiverDocument(registryPath) {
  try {
    return JSON.parse(fs.readFileSync(registryPath, "utf8"));
  } catch (error) {
    throw new Error(`Cannot load packaged waiver registry ${registryPath}: ${error.message}`);
  }
}
