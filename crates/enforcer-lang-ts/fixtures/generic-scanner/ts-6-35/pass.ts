export function toWidget(dto: WidgetDto): Widget {
  return { id: dto.id, name: dto.name };
}
