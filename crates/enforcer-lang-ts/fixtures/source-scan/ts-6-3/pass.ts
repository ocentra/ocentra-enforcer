import { Schema } from "effect";

export function widget(raw: unknown): Widget {
  return Schema.decodeUnknownSync(WidgetSchema)(raw);
}
const detail = "as separate registration steps";
