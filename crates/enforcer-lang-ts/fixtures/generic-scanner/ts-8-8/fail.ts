test("saves a widget", () => {
  jest.mock("./widget-repository");
  saveWidget(widget);
});
