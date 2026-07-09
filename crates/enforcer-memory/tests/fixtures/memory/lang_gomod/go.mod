module example.com/foo

go 1.21

require github.com/bar/baz v1.2.3

require (
	github.com/x/y v0.1.0
)

replace github.com/bar/baz => ../baz
