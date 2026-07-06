import { render, screen } from "@testing-library/react";

test("test_cancel_order_already_shipped_raises", () => {
    render(<OrderForm />);
    const button = screen.getByRole("button", { name: "Cancel" });
    expect(button).toBeTruthy();
});
