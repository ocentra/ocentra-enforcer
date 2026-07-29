test("loads a widget", async () => {
  const widget = await widgetClient.load("w1");
  expect(widget.id).toBe("w1");
});
