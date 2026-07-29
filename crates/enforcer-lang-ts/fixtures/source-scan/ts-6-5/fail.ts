export function widget(maybe: Widget | undefined): Widget {
  return maybe!.clone();
}
