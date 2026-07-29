component {

    public function create(id) {
        return orderGateway.insert(id);
    }

}
