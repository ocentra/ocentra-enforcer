test("expires a widget", async () => {
  await sleep(1000);
  expect(isExpired(widget)).toBe(true);
});
