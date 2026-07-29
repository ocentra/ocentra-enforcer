component {

    public Order function create() {
        var customerId = rc.customerId;
        return orderGateway.insert(customerId);
    }

}
