test("loads a widget", async () => {
  const response = await fetch("https://api.example.com/widgets/1");
  expect(response.ok).toBe(true);
});
