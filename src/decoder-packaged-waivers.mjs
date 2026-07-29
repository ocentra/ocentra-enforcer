//! Decoder boundary for the on-disk packaged-waiver document.
// BOUNDARY-INVARIANT: raw file bytes enter only here and the caller immediately
// validates the document's waiver array before any record is used.
import fs from "node:fs";
import { PackagedWaiverDocumentReadError } from "./error-packaged-waiver-document-read.mjs";
import { MalformedPackagedWaiverDocumentError } from "./error-malformed-packaged-waiver-document.mjs";

/** Decode untrusted packaged-waiver JSON at the filesystem boundary. */
export function decodePackagedWaiverDocument(registryPath) {
  let source;
  try {
    source = fs.readFileSync(registryPath, "utf8");
  } catch (error) {
    throw new PackagedWaiverDocumentReadError(registryPath, error);
  }

  try {
    return JSON.parse(source);
  } catch (error) {
    throw new MalformedPackagedWaiverDocumentError(registryPath, error);
  }
}
