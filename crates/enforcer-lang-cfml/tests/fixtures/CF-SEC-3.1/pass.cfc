component {

    property name="log" inject="Logbox:logger:{this}";

    public struct function create(required struct data) {
        try {
            return doCreate(data);
        } catch (any e) {
            log.error("order create failed", e);
            return { success: false, message: "unable to create order" };
        }
    }

}
