component {

    public struct function create(required struct data) {
        try {
            return doCreate(data);
        } catch (any e) {
            return { success: false, detail: e.tagContext, stack: cfcatch.tagContext };
        }
    }

}
