component {

    public void function validate(required numeric id) {
        if (id lte 0) {
            throw(message="bad");
        }
    }

}
