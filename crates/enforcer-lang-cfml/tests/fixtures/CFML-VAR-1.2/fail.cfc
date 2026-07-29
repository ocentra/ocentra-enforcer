component {

    public numeric function getId(required numeric id) {
        if (arguments.id lte 0) {
            throw(type="app.validation.invalidOrder", message="id must be positive");
        }
        return id;
    }

}
