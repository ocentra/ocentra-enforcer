use std::fs;

struct Widget;

trait Drawable {
    fn draw(&self);
}

fn render(widget: &Widget) {
    fs::read_to_string("widget.txt").ok();
    helper();
}

fn helper() {}

#[test]
fn render_does_not_panic() {
    render(&Widget);
}
