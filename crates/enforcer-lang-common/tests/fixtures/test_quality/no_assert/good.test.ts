import { svc } from "./svc";

it("processes the order", () => {
    const result = svc.do();
    expect(result).toEqual({ status: "processed" });
});
