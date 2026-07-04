test("renders a widget", () => {
  expect(renderWidget(widget)).toEqual({ id: "w1", label: "Widget" });
});
