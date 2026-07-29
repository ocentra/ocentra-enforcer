contract;

use std::storage::storage_api::*;

struct Widget {
    label: str,
}

trait Drawable {
    fn draw(self);
}

impl Drawable for Widget {
    fn draw(self) {
        helper(self.label);
    }
}

fn helper(label: str) {
    if label.len() > 0 {
        log(label);
    }
}
