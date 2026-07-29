cmake_minimum_required(VERSION 3.10)
project(Widget)

include(cmake/helpers.cmake)
add_subdirectory(src)

function(greet name)
  message("hello ${name}")
endfunction()

macro(setup)
  greet("world")
endmacro()

if(WIN32)
  message("on windows")
endif()
