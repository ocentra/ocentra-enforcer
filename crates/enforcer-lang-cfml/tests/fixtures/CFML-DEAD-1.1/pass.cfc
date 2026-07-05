component {

    public numeric function total(required array items) {
        var runningTotal = 0;
        for (item in arguments.items) {
            runningTotal += item.price;
        }
        return runningTotal;
    }

}
