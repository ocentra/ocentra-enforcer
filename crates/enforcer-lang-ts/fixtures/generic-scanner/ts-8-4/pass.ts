describe("Widget", () => {
  test("creates a widget with the given id", () => {
    expect(createWidget("w1").id).toBe("w1");
  });
});
