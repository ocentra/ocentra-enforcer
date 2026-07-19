import { functionDefinitions } from "./rust-rules-source-test-evidence-persistence-definitions.mjs";
import { readerTargets, writerTargets } from "./rust-rules-source-test-evidence-persistence-targets.mjs";

/** Describes persistence writer/reader pairs only when both sides use a real codec. */
export function roundTripPersistenceDescriptors(source, masked = undefined) {
  const definitions = functionDefinitions(source, masked);
  const writers = definitions.flatMap((definition) =>
    writerTargets(definition).map((targetName) => ({
      targetName,
      writer: definition.name,
    })));
  const readers = definitions.flatMap((definition) =>
    readerTargets(definition).map((targetName) => ({
      targetName,
      reader: definition.name,
    })));
  return writers.flatMap((writer) =>
    readers.filter((reader) => reader.targetName === writer.targetName)
      .map((reader) => ({ ...writer, reader: reader.reader })));
}
