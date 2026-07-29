const std = @import("std");

const Widget = struct {
    name: []const u8,

    fn draw(self: Widget) void {
        if (self.name.len == 0) {
            std.debug.print("unnamed", .{});
        }
        std.debug.print("{s}", .{self.name});
    }
};

const Color = enum {
    Red,
    Green,
};

fn helper(label: []const u8) []const u8 {
    return label;
}

test "widget draws its name" {
    var w = Widget{ .name = "test" };
    w.draw();
}

pub fn main() void {
    helper("hi");
}
