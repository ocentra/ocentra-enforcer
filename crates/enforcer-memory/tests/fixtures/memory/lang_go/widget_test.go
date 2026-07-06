package widget

import "testing"

func TestNewWidget(t *testing.T) {
	w := NewWidget("foo")
	if w.Name != "foo" {
		t.Fatal("unexpected name")
	}
}

func helperNotATest() {
	// Not a Test-prefixed name, must not be classified as a Test symbol.
}
