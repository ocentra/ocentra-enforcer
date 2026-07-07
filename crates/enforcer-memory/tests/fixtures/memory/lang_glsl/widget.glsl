#version 330 core
#include "common.glsl"

struct Widget {
    vec3 position;
    vec3 color;
};

vec3 helper(vec3 x) {
    if (x.x == 0.0) {
        return x;
    }
    return x * 2.0;
}

void main() {
    vec3 result = helper(vec3(1.0, 0.0, 0.0));
}
