#include "common.isph"

struct Particle {
    float x;
    float y;
};

float add(float a, float b) {
    return a + b;
}

export void updateAll(uniform float pos[], uniform int n) {
    foreach (i = 0 ... n) {
        float r = add(pos[i], 1.0f);
        if (r > 0.0f) {
            pos[i] = r;
        }
    }
}
