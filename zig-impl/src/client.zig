const std = @import("std");
const IoUring = std.os.linux.IoUring;
const ffi = @import("c.zig");
const linux = std.os.linux;
const assert = std.debug.assert;

const ClientFSM = enum { Idle, IsConnecting, IsUpgradingTLS, IssueRead, IsReading };

const Context = struct {
    allr: std.mem.Allocator,
    uring: IoUring,
    sockfd: linux.fd_t,
    state: ClientFSM,
};

const ClientSettings = struct {
    tls: bool,
    blocking: bool,
};

const Client = @This();
settings: ClientSettings,
context: Context,
ssl: ?*ffi.WOLFSSL,
wolfssl_ctx: ?*ffi.WOLFSSL_CTX, // TODO naming, this may not be required.

pub fn init(comptime settings: ClientSettings, queue_depth: u16) !Client {
    // const uring =
    // const sockfd = ;
    var client = Client{
        .settings = settings,
        .ssl = null, // this is set below in wolfSSLInit
        .wolfssl_ctx = null, // this is set below in wolfSSLInit
        .context = .{
            .allr = std.heap.raw_c_allocator, // TODO
            .state = ClientFSM.Idle,
            .uring = try IoUring.init(queue_depth, 0),
            .sockfd = @intCast(linux.socket(linux.AF.INET, linux.SOCK.STREAM | linux.SOCK.NONBLOCK, linux.IPPROTO.TCP)),
        },
    };
    try wolfSSLInit(&client); // TODO only if wss
    return client;
}

// TODO is blocking
pub fn connect(cc: *Client, url: []const u8) !bool {
    // std.debug.print("connect: {}\n", .{cc.ctx.state});
    switch (cc.context.state) {
        .Idle => {
            const uri = try std.Uri.parse(url);
            // TODO should support unsecure connection (ws), just dont read/write bytes to TLS buffers
            // and directly return it to user.
            // assert(std.mem.eql(u8, uri.scheme, "wss")); // Only TLS is accepted

            const addresses = try std.net.getAddressList(cc.context.allr, uri.host.?.percent_encoded, 443);
            defer addresses.deinit();
            const ipv4 = addresses.addrs[0];
            const sqe = try cc.context.uring.get_sqe();

            sqe.prep_connect(cc.context.sockfd, &ipv4.any, ipv4.getOsSockLen());
            const conn_ret = try cc.context.uring.submit();
            if (conn_ret < 0) {
                return error.CouldNotSubmit;
            }
            cc.context.state = ClientFSM.IsConnecting;
            return false;
        },
        .IsConnecting => {
            // std.debug.print("{}\n", .{cc.ctx.uring.sq_ready()});
            const entries = IoUring.cq_ready(&cc.context.uring);
            if (entries > 0) {
                IoUring.cq_advance(&cc.context.uring, entries);
                if (cc.settings.tls) {
                    cc.context.state = .IsUpgradingTLS;
                } else {
                    return true;
                }
            }
            return false;
        },
        .IsUpgradingTLS => {
            cc.context.state = .IssueRead;
            std.debug.print("{}\n", .{cc});
            std.debug.print("{?} {?}\n", .{ cc.ssl, cc.wolfssl_ctx });
            const ret = ffi.wolfSSL_connect(cc.ssl);
            std.debug.print("ERR CODE: {}\n", .{ffi.wolfSSL_get_error(cc.ssl, ret)});
            switch (ret) {
                ffi.SSL_SUCCESS => return true,
                ffi.SSL_ERROR_WANT_READ, ffi.SSL_ERROR_WANT_WRITE => return false,
                else => {
                    std.debug.print("ret: {}\n", .{ret});
                    return error.WolfSSLConnectError;
                },
            }
        },
        else => {
            assert(false); // TODO
            return false;
        },
    }
}

// fn clientRingInit(queue_depth: u16) !IoUring {
//     std.debug.assert(std.math.isPowerOfTwo(queue_depth));
//     return try IoUring.init(queue_depth, 0);
// }

fn wolfSSLInit(cc: *Client) !void {
    _ = ffi.wolfSSL_Init();

    // maybe stack variable is fine, ie const ctx = ...
    cc.wolfssl_ctx = ffi.wolfSSL_CTX_new(ffi.wolfSSLv23_client_method()).?;
    ffi.wolfSSL_SetIORecv(cc.wolfssl_ctx.?, recvCallback);
    ffi.wolfSSL_SetIOSend(cc.wolfssl_ctx.?, sendCallback);

    ffi.wolfSSL_CTX_set_verify(cc.wolfssl_ctx, ffi.WOLFSSL_VERIFY_NONE, null);
    cc.ssl = ffi.wolfSSL_new(cc.wolfssl_ctx).?;
    ffi.wolfSSL_SetIOReadCtx(cc.ssl, &cc.context);
    ffi.wolfSSL_SetIOWriteCtx(cc.ssl, &cc.context);
}

fn recvCallback(ssl: ?*ffi.WOLFSSL, buf: [*c]u8, sz: c_int, ctx: ?*anyopaque) callconv(.C) c_int {
    std.debug.print("in read callback\n", .{});
    _ = ssl; // autofix
    var cc: *Client = @ptrCast(@alignCast(ctx));

    if (cc.context.state == .IssueRead) {
        clientPrepRead(cc, sz) catch |err| {
            // TODO
            std.debug.print("{}", .{err});
            assert(false);
        };
        cc.context.state = .IsReading;
    }

    const in_queue = cc.context.uring.cq_ready();
    if (in_queue == 0) {
        return ffi.WOLFSSL_CBIO_ERR_WANT_READ;
    } else {
        const cqe = peek_cqe(&cc.context.uring) catch |err| blk: {
            std.debug.print("{}", .{err});
            assert(false);
            break :blk null;
        };
        const data: *const []u8 = @ptrFromInt(cqe.?.user_data);
        const mutable_buf = @as([*]u8, @ptrCast(buf))[0..data.*.len];
        std.mem.copyForwards(u8, mutable_buf, data.*);

        cc.context.state = .IssueRead;
        return cqe.?.res;
    }
}

fn clientPrepRead(cc: *Client, sz: c_int) !void {
    const sqe = try IoUring.get_sqe(&cc.context.uring);
    const buf = try cc.context.allr.alloc(u8, @intCast(sz));
    sqe.prep_read(cc.context.sockfd, buf, 0);
    const s = try cc.context.uring.submit();
    assert(s >= 0);
}

fn sendCallback(ssl: ?*ffi.WOLFSSL, buf: [*c]u8, sz: c_int, ctx: ?*anyopaque) callconv(.C) c_int {
    _ = sz; // autofix
    _ = buf; // autofix
    std.debug.print("in send callback {?}\n", .{ssl});
    const context: *Context = @ptrCast(@alignCast(ctx));
    std.debug.print("callback {}\n", .{context.uring});
    assert(false);
    return 0;
    // clientPrepSend(context, buf, sz) catch |err| {
    //     // TODO
    //     std.debug.print("{}", .{err});
    //     assert(false);
    // };
    // const completed = context.uring.cq_ready();
    // if (completed > 0) {
    //     const cqe = peek_cqe(&context.uring) catch |err| blk: {
    //         std.debug.print("{}", .{err});
    //         assert(false);
    //         break :blk null;
    //     };
    //     return cqe.?.res;
    // } else {
    //     assert(completed == 0);
    //     return 0;
    // }
}

fn clientPrepSend(ctx: *Context, buf: [*c]u8, sz: c_int) !void {
    // std.debug.print("{}\n", .{ctx.uring});
    std.debug.print("{}\n", .{ctx.uring.sq_ready()});
    const sqe = try ctx.uring.get_sqe();
    std.debug.print("{}\n", .{sqe});
    // Cast a *char to a u8 slice, then make const.
    const slice: []const u8 = @as([*]u8, @ptrCast(buf))[0..@intCast(sz)];
    std.debug.print("{s}\n", .{slice});
    sqe.prep_send(ctx.sockfd, slice, 0);
    _ = try ctx.uring.submit();
}

fn peek_cqe(ring: *IoUring) !?linux.io_uring_cqe {
    var cqes: [1]linux.io_uring_cqe = undefined;
    const count = try ring.copy_cqes(&cqes, 1);
    if (count > 0) return cqes[0];
    return null;
}
