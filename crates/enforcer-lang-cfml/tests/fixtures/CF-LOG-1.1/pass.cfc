component {

    property name="log" inject="Logbox:logger:{this}";

    public void function debugOrder(required struct order) {
        log.error("order debug", order);
    }

}
