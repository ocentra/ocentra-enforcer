component {

    property name="log" inject="Logbox:logger:{this}";

    public void function create(required struct data) {
        try {
            doCreate(data);
        } catch(any e) {
            log.error("order create failed", e);
            rethrow;
        }
    }

}
