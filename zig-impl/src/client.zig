const std = @import("std");
const IoUring = std.os.linux.IoUring;
const c = @import("c.zig");
const linux = std.os.linux;
const assert = std.debug.assert;

const ClientFSM = enum { Idle, IsConnecting, ShouldRecieve, HasBytes };

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
    // const flags = linux.fcntl(@intCast(sockfd), linux.F.GETFL, 0);
    // sockfd = linux.fcntl(@intCast(sockfd), linux.F.SETFL, flags | linux.O.NONBLOCK);
    const client = Client{
        .allr = std.heap.raw_c_allocator, // TODO
        .ctx = .{
            .state = ClientFSM.Idle,
            .uring = uring,
            .sockfd = @intCast(sockfd),
        },
    };
    return client;
}

// TODO is blocking
pub fn connect(cc: *Client, url: []const u8) !bool {
    switch (cc.ctx.state) {
        .Idle => {
            const uri = try std.Uri.parse(url);
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

fn wolfSSLInit() !void {
    c.wolfSSL_Init();

    const ctx = c.wolfSSL_CTX_new(c.wolfSSLv23_client_method()).?;
    c.wolfSSL_SetIORecv(ctx, recvCallback);
    c.wolfSSL_SetIOSend(ctx, sendCallback);
}

fn recvCallback(ssl: *c.WOLFSSL, buf: *c_char, sz: c_int, ctx: *anyopaque) void {
    _ = ssl; // autofix
    const cc: *Client = @ptrCast(ctx);
    // var ret = 0;
    if (cc.ctx.should_prep) {
        clientPrepRead(cc, sz);
    }

    const in_queue = try cc.ctx.uring.cq_ready();
    if (in_queue == 0) {
        cc.ctx.should_prep = false;
        return c.WOLFSSL_CBIO_ERR_WANT_READ;
    } else {
        const cqe = try cc.ctx.uring.copy_cqe(); // TODO this blocks
        @memcpy(buf, cqe.user_data);
        cc.ctx.should_prep = true;
        return cqe.res;
    }
}

fn clientPrepRead(cc: *Client, sz: c_int) !void {
    const sqe = try IoUring.get_sqe(cc.ctx.uring);
    const buf = try cc.allr.alloc(u8, sz);
    sqe.prep_read(cc.sockfd, buf, 0);
    cc.ctx.uring.submit();
}

fn sendCallback(ssl: *c.WOLFSSL, buf: *c_char, sz: c_int, ctx: *anyopaque) void {
    _ = sz; // autofix
    _ = buf; // autofix
    _ = ssl; // autofix
    const cc: *Client = @ptrCast(ctx);
    clientPrepSend();
    const completed = try cc.ctx.uring.cq_ready();
    if (completed > 0) {
        const cqe = try cc.ctx.uring.copy_cqe(); // TODO this blocks
        return cqe.res;
    } else {
        assert(completed == 0);
        return 0;
    }
}

fn clientPrepSend(cc: *Client, buf: *c_char, sz: c_int) void {
    _ = sz; // autofix
    const sqe = try cc.ctx.uring.get_sqe();
    // Cast a *char to a u8 slice, then make const.
    const slice: []const u8 = @as(*u8, @ptrCast(buf))[0..std.mem.len(buf)];
    sqe.prep_send(cc.ctx.sockfd, slice, 0);
    cc.ctx.uring.submit();
}
