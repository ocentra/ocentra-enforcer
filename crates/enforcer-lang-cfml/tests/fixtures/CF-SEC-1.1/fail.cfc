component {

    public query function findById(required numeric id) {
        return queryExecute("SELECT * FROM orders WHERE id = #arguments.id#");
    }

}
