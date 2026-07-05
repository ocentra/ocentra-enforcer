component {

    public numeric function total(required array items) {
        var runningTotal = 0;
        for (item in arguments.items) {
            var lineTotal = item.price;
            runningTotal += lineTotal;
        }
        return runningTotal;
    }

}
