export function toWidget(raw: unknown): Widget {
  return Schema.decodeUnknownSync(WidgetSchema)(raw);
}
