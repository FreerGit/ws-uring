const std = @import("std");

const Client = @import("client.zig");

pub fn print(comptime fmt: []const u8, args: anytype) void {
    std.debug.print(fmt ++ "\n", args);
}

pub fn main() !void {
    print("{s}", .{"hello world"});
    var client = try Client.init(2);
    const begin = std.time.microTimestamp();
    while (true) {
        const connected = try Client.connect(&client, "wss://stream.bybit.com/smtg");
        print("{}", .{connected});
        if (connected) break;
    }
    const end = std.time.microTimestamp();
    print("{d}", .{end - begin});
}

test "simple test" {
    var list = std.ArrayList(i32).init(std.testing.allocator);
    defer list.deinit(); // try commenting this out and see if zig detects the memory leak!
    try list.append(42);
    try std.testing.expectEqual(@as(i32, 42), list.pop());
}
