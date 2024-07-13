const std = @import("std");
const IoUring = std.os.linux.IoUring;
const ffi = @import("c.zig");
const linux = std.os.linux;
const assert = std.debug.assert;

const ClientFSM = enum { Idle, IsConnecting, IssueRead, IsReading };

const Ctx = struct {
    uring: IoUring,
    sockfd: linux.fd_t,
    state: ClientFSM,
};

const Client = @This();
allr: std.mem.Allocator,
ctx: Ctx,

pub fn init(queue_depth: u16) !Client {
    const uring = try clientRingInit(queue_depth);
    const sockfd = linux.socket(linux.AF.INET, linux.SOCK.STREAM | linux.SOCK.NONBLOCK, linux.IPPROTO.TCP);
    var client = Client{
        .allr = std.heap.raw_c_allocator, // TODO
        .ctx = .{
            .state = ClientFSM.Idle,
            .uring = uring,
            .sockfd = @intCast(sockfd),
        },
    };
    try wolfSSLInit(&client); // TODO only if wss
    return client;
}

// TODO is blocking
pub fn connect(cc: *Client, url: []const u8) !bool {
    switch (cc.ctx.state) {
        .Idle => {
            const uri = try std.Uri.parse(url);
            // TODO should support unsecure connection (ws), just dont read/write bytes to TLS buffers
            // and directly return it to user.
            assert(std.mem.eql(u8, uri.scheme, "wss")); // Only TLS is accepted

            const addresses = try std.net.getAddressList(cc.allr, uri.host.?.percent_encoded, 443);
            defer addresses.deinit();
            const ipv4 = addresses.addrs[0];
            const sqe = try cc.ctx.uring.get_sqe();

            sqe.prep_connect(cc.ctx.sockfd, &ipv4.any, ipv4.getOsSockLen());
            const conn_ret = try cc.ctx.uring.submit();
            if (conn_ret < 0) {
                return error.CouldNotSubmit;
            }
            cc.ctx.state = ClientFSM.IsConnecting;
            return false;
        },
        .IsConnecting => {
            const ready = IoUring.cq_ready(&cc.ctx.uring);
            if (ready > 0) {
                assert(ready == 1); // TODO this _shouldn't_ happen, dev.
                std.debug.print("{}", .{cc.ctx.uring.cq.cqes[ready]});
                return true;
            }
            std.debug.print("Done \n", .{});
            return false;
        },
        else => {
            assert(false); // TODO
            return false;
        },
    }
}

fn clientRingInit(queue_depth: u16) !IoUring {
    std.debug.assert(std.math.isPowerOfTwo(queue_depth));
    return try IoUring.init(queue_depth, 0);
}

fn wolfSSLInit(cc: *Client) !void {
    _ = ffi.wolfSSL_Init();

    const ctx = ffi.wolfSSL_CTX_new(ffi.wolfSSLv23_client_method()).?;
    ffi.wolfSSL_SetIORecv(ctx, recvCallback);
    ffi.wolfSSL_SetIOSend(ctx, sendCallback);

    ffi.wolfSSL_CTX_set_verify(ctx, ffi.WOLFSSL_VERIFY_NONE, null);
    const ssl = ffi.wolfSSL_new(ctx).?;
    ffi.wolfSSL_SetIOReadCtx(ssl, &cc.ctx);
}

fn recvCallback(ssl: ?*ffi.WOLFSSL, buf: [*c]u8, sz: c_int, ctx: ?*anyopaque) callconv(.C) c_int {
    _ = ssl; // autofix
    var cc: *Client = @alignCast(@ptrCast(ctx));

    if (cc.ctx.state == .IssueRead) {
        clientPrepRead(cc, sz) catch |err| {
            // TODO
            std.debug.print("{}", .{err});
            assert(false);
        };
        cc.ctx.state = .IsReading;
    }

    const in_queue = cc.ctx.uring.cq_ready();
    if (in_queue == 0) {
        return ffi.WOLFSSL_CBIO_ERR_WANT_READ;
    } else {
        // TODO this blocks
        const cqe = peek_cqe(&cc.ctx.uring) catch |err| blk: {
            std.debug.print("{}", .{err});
            assert(false);
            break :blk null;
        };
        // -.-
        const data: *const []u8 = @ptrFromInt(cqe.?.user_data);
        const mutable_buf = @as([*]u8, @ptrCast(buf))[0..data.*.len];
        std.mem.copyForwards(u8, mutable_buf, data.*);

        cc.ctx.state = .IssueRead;
        return cqe.?.res;
    }
}

fn clientPrepRead(cc: *Client, sz: c_int) !void {
    const sqe = try IoUring.get_sqe(&cc.ctx.uring);
    const buf = try cc.allr.alloc(u8, @intCast(sz));
    sqe.prep_read(cc.ctx.sockfd, buf, 0);
    const s = try cc.ctx.uring.submit();
    assert(s >= 0);
}

fn sendCallback(ssl: ?*ffi.WOLFSSL, buf: [*c]u8, sz: c_int, ctx: ?*anyopaque) callconv(.C) c_int {
    _ = ssl; // autofix
    const cc: *Client = @alignCast(@ptrCast(ctx));
    clientPrepSend(&cc.ctx, buf, sz) catch |err| {
        // TODO
        std.debug.print("{}", .{err});
        assert(false);
    };
    const completed = cc.ctx.uring.cq_ready();
    if (completed > 0) {
        const cqe = peek_cqe(&cc.ctx.uring) catch |err| blk: {
            std.debug.print("{}", .{err});
            assert(false);
            break :blk null;
        };
        return cqe.?.res;
    } else {
        assert(completed == 0);
        return 0;
    }
}

fn clientPrepSend(ctx: *Ctx, buf: [*c]u8, sz: c_int) !void {
    const sqe = try ctx.uring.get_sqe();
    // Cast a *char to a u8 slice, then make const.
    const slice: []const u8 = @as([*]u8, @ptrCast(buf))[0..@intCast(sz)];
    sqe.prep_send(ctx.sockfd, slice, 0);
    _ = try ctx.uring.submit();
}

fn peek_cqe(ring: *IoUring) !?linux.io_uring_cqe {
    var cqes: [1]linux.io_uring_cqe = undefined;
    const count = try ring.copy_cqes(&cqes, 1);
    if (count > 0) return cqes[0];
    return null;
}
