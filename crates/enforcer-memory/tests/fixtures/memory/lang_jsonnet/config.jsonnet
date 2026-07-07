local greeting(name) = "hello " + name;
local isProd = true;
{
  message: greeting("world"),
  value: if isProd then 1 else 2,
  imported: import "other.libsonnet",
  fn: function(x) x + 1,
}
