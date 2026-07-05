component {

    public Order function create(required struct data) {
        var svc = new OrderService();
        return svc.create(data);
    }

}
