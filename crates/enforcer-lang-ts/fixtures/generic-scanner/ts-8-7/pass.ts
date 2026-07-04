test("expires a widget", async () => {
  await clock.advanceBy(1000);
  expect(isExpired(widget)).toBe(true);
});
