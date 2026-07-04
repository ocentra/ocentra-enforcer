import { WidgetRepository } from "../infrastructure/widget-repository";

export function loadWidget(id: string): WidgetRepository {
  return new WidgetRepository();
}
