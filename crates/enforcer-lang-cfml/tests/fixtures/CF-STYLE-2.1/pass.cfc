component {

    public Order function create(required numeric id) {
        return orderGateway.insert(id);
    }

}
