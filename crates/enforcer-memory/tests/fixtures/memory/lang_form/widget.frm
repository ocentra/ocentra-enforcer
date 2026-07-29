#include "widget_defs.h"

Symbols x, y;

id F = x;

#procedure greet(x)
  id x = 1;
  #call draw(x)
#endprocedure

#call greet(x)
