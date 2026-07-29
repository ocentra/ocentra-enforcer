component {

    property name="orderGateway" inject="OrderGateway";

    public Order function create(required struct data) {
        var result = orderGateway.insert(data);
        return result;
    }

}
