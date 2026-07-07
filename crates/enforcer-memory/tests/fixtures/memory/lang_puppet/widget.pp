class widget {
  $name = "foo"

  if $name == "foo" {
    notify { "found": }
  }

  helper($name)
}

define helper($x) {
  notify { "helper: ${x}": }
}

include stdlib
