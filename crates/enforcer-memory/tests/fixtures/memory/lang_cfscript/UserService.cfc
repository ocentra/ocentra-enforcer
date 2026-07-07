component {
    property name="userId" type="string";

    function getUser(required string id) {
        var result = queryUser(id);
        if (result.count > 0) {
            return result[1];
        } else {
            return {};
        }
    }

    private function queryUser(string id) {
        return userService.find(id);
    }
}
