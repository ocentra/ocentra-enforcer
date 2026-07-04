describe("Widget", () => {
  test("creates a widget with the given id", () => {
    const widget = createWidget("w1");
    expect(widget.id).toBe("w1");
  });
});
