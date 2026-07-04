import { Schema } from "effect";

export function decode(raw: string): unknown {
  return Schema.decodeUnknownSync(Schema.Unknown)(raw);
}
