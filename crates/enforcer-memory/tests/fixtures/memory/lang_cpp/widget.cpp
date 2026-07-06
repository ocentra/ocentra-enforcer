#include <string>
#include "widget.h"

namespace widgets {

class Widget {
public:
    Widget();
    std::string draw() const;
private:
    std::string name_;
};

std::string Widget::draw() const {
    return name_;
}

class Drawable {
public:
    virtual std::string draw() const = 0;
    virtual ~Drawable() = default;
};

class Base {
public:
    int id;
};

class DerivedWidget : public Base {
public:
    void render() {}
};

void helper_fn() {}

}  // namespace widgets
