component {

    public numeric function total(required array items) {
        runningTotal = 0;
        for (item in arguments.items) {
            runningTotal = runningTotal + item.price;
        }
        return runningTotal;
    }

}
