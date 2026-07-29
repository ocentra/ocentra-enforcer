package com.example.widget;

import java.util.List;
import java.util.ArrayList;

public interface Drawable {
    String draw();
}

public abstract class Shape implements Drawable {
    public abstract String draw();
}

public class Widget extends Shape {
    public static final int MAX_WIDGETS = 10;

    private String name;

    public Widget(String name) {
        this.name = name;
    }

    @Override
    public String draw() {
        return String.format("widget:%s", this.name);
    }
}
