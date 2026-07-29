enable f16;

struct Widget {
    label: f32,
};

fn helper(label: f32) -> f32 {
    if (label > 0.0) {
        return label;
    }
    return 0.0;
}

fn draw(label: f32) -> f32 {
    return helper(label);
}
