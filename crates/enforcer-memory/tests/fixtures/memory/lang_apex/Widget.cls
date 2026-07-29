public class Widget extends BaseWidget implements Drawable {
    public String name;

    public Widget(String name) {
        this.name = name;
    }

    public String draw() {
        if (name == null) {
            return helper('unnamed');
        }
        return helper(name);
    }

    private String helper(String label) {
        System.debug(label);
        return label;
    }
}
