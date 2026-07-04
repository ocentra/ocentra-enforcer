export async function loadWidget(): Promise<Widget> {
  const mod = await import("./widget-impl");
  return mod.createWidget();
}
