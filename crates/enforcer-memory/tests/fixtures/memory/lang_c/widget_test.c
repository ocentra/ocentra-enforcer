#include "widget.h"

void test_widget_new_sets_id() {
    struct Widget* w = widget_new("foo");
}

void helper_not_a_test() {
    /* Not a test_-prefixed name, must not be classified as a Test symbol
       when the file itself is not on a *_test.c path -- but this file IS
       named widget_test.c, so the file-level signal promotes it anyway. */
}
