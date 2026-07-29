component {

    public Order function create(required struct data) {
        return application.orderService.create(data);
    }

}
