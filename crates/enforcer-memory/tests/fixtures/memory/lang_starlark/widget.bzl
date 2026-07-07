load("//tools/build_defs:widget.bzl", "widget_library")

def helper(name):
    if name:
        return name
    return "unnamed"

def draw(name):
    label = helper(name)
    widget_library(
        name = label,
        srcs = ["widget.bzl"],
    )
