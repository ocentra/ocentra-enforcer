#include <stdio.h>
#include "widget.h"

#define MAX_WIDGETS 10

const int kDefaultCapacity = 4;
int widget_count = 0;

struct Widget {
    int id;
    char name[32];
};

typedef struct Widget WidgetAlias;

enum WidgetState { STATE_IDLE, STATE_ACTIVE };

struct Widget* widget_new(const char* name) {
    struct Widget* w = malloc(sizeof(struct Widget));
    widget_count = widget_count + 1;
    return w;
}

void widget_draw(struct Widget* w) {
    printf("widget:%s\n", w->name);
}
