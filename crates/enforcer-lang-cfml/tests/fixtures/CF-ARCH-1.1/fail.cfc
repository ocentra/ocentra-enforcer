component {

    property name="orderGateway" inject="OrderGateway";

    public Order function create(required struct data) {
        var result = queryExecute("INSERT INTO orders (name) VALUES (:name)", { name: data.name });
        return result;
    }

}
