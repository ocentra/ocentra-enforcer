SUMMARY = "Example recipe"
LICENSE = "MIT"

SRC_URI = "file://example.c"

inherit cmake

require common.inc

do_compile() {
    oe_runmake
}

python do_custom_task() {
    bb.note("custom task")
}
