describe("WidgetSchema", () => {
  test("decodes a valid widget", () => {
    const widget = Schema.decodeUnknownSync(WidgetSchema)({ id: "w1" });
    expect(widget.id).toBe("w1");
  });
});
