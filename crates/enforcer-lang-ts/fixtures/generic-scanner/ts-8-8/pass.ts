test("saves a widget", () => {
  const repository = new InMemoryWidgetRepository();
  saveWidget(widget, repository);
  expect(repository.find(widget.id)).toBeDefined();
});
