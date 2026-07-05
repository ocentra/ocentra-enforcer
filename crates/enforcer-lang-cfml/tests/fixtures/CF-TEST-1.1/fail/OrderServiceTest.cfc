component extends="SomeOtherBase" {

    function run() {
        describe("OrderService", function() {
            it("creates an order", function() {
                expect(orderService.create({})).toBeInstanceOf("Order");
            });
        });
    }

}
