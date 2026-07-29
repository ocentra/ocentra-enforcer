export function run(command: WidgetCommand): WidgetResult {
  return dispatchWidgetCommand(command);
}
