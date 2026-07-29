import { render } from "@testing-library/react";

test("test_order_1", () => {
    const { container } = render(<OrderForm />);
    const button = container.querySelector("[data-testid=submit]");
    expect(button).toBeTruthy();
});
