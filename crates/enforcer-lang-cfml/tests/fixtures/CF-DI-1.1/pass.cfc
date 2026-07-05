component {

    property name="orderService" inject="OrderService";

    public Order function create(required struct data) {
        return orderService.create(data);
    }

}
