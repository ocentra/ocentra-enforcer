component {

    public void function validate(required numeric id) {
        if (id lte 0) {
            throw(type="app.validation.invalidOrder", message="id must be positive");
        }
    }

}
