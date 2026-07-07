import QtQuick
import "./helpers.js" as Helpers

Item {
    id: root

    property int count: 0
    signal clicked(int x)

    component Circle: Rectangle {
        property int radius: 50
    }

    function increment() {
        count = count + 1;
        console.log("incrementing");
        Helpers.notify(count);
    }

    function reset() {
        if (count > 0) {
            count = 0;
        } else {
            console.log("already zero");
        }
        for (let i = 0; i < 3; i++) {
            console.log(i);
        }
        while (count < 0) {
            count++;
        }
    }

    function makeHelper() {
        class Helper {
            label = "hi";

            draw() {
                return this.label;
            }
        }
        let h = new Helper();
        h.draw();
        return h;
    }
}
