@0xdbb9ad1d10a10a1f;

using Cxx = import "/capnp/c++.capnp";

const globalConst :Int32 = 42;

struct Person {
  name @0 :Text;
  age @1 :Int32 = 0;

  struct Nested {
    x @0 :Int32;
  }
}

interface Greeter {
  greet @0 (name :Text) -> (reply :Text);
}
