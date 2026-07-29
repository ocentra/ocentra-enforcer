namespace cpp my.namespace

include "other.thrift"

struct Person {
  1: string name,
  2: i32 age = 0,
}

exception MyError {
  1: string message,
}

service Greeter {
  string greet(1: string name),
}
