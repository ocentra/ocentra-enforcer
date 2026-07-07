import 'dart:async';
import 'package:foo/bar.dart' as bar;

class Base {
  int id = 0;
}

abstract class Drawable {
  String draw();
}

class Widget extends Base implements Drawable {
  String name = "widget";

  @override
  String draw() {
    if (name.isEmpty) {
      return "x";
    }
    for (var i = 0; i < 3; i++) {
      helper();
    }
    return bar.render(name);
  }
}

enum Status { active, inactive }

typedef IntCallback = void Function(int x);

void helper() {}

void topLevel() {
  var w = Widget();
  w.draw();
}
