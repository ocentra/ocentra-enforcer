#include "widget.h"

TEST(WidgetSuite, DrawsName) {
    widgets::Widget w;
}

void helper_not_a_test() {
    // Not a test_-prefixed name and no TEST() macro -- but this file IS
    // named widget_test.cpp, so the file-level signal promotes it anyway.
}
