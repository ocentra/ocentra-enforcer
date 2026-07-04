test("creates a widget", () => {
  expect(createWidget()).toEqual({ id: expect.any(String) });
});
