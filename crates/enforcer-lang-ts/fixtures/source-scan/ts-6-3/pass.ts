import { Schema } from "effect";

export function widget(raw: unknown): Widget {
  return Schema.decodeUnknownSync(WidgetSchema)(raw);
}
