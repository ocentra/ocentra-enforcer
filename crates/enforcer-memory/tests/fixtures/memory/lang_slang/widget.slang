import WidgetLib;

struct Widget {
    float label;
};

float helper(float label) {
    if (label > 0.0) {
        return label;
    }
    return 0.0;
}

float draw(float label) {
    return helper(label);
}
