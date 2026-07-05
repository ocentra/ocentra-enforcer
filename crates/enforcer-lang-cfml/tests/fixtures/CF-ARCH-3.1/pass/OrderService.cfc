component {

    public Order function create(required numeric customerId) {
        return orderGateway.insert(customerId);
    }

}
