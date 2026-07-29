package widget

import (
	"fmt"
	"net/http"
)

// Base is embedded into Widget below -- best-effort INHERITS source.
type Base struct {
	ID int
}

type Widget struct {
	Base
	Name string
}

type Drawable interface {
	Draw() string
}

const MaxWidgets = 10

var registry = map[string]int{}

func NewWidget(name string) *Widget {
	return &Widget{Name: name}
}

func (w *Widget) Draw() string {
	return fmt.Sprintf("widget:%s", w.Name)
}

func RegisterRoutes(mux *http.ServeMux) {
	mux.HandleFunc("/widgets", ListWidgets)
}

func ListWidgets(w http.ResponseWriter, r *http.Request) {
	fmt.Println("listing")
}
