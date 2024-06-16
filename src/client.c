#include "client.h"

#include <fcntl.h>

#include "base.h"

FN_PURE char *
get_client_error(int error_code) {
  switch (error_code) {
    case 1:
      return "Could not initialize io_uring, kernel version problem?";
    case 2:
      return "Could not set socket to non-blocking";
    default:
      return "Not an url error, probably using the wrong get_*_error() "
             "function";
  }
}

static int
client_ring_init(client_t *client, int queue_depth) {
  if (UNLIKELY(io_uring_queue_init(queue_depth, &client->ctx.ring, 0) < 0)) {
    return 1;
  }
  return 0;
}

FN_PURE static int
set_socket_nonblocking(int sockfd) {
  int flags = fcntl(sockfd, F_GETFL, 0);

  if (UNLIKELY(flags < 0 || fcntl(sockfd, F_SETFL, flags | O_NONBLOCK) < 0)) {
    return -1;
  }
  return 0;
}

int
client_init(client_t *client, int queue_depth) {
  int signal = client_ring_init(client, 1024 * 8);
  if (UNLIKELY(signal != 0)) {
    return signal;
  }

  int sockfd = socket(AF_INET, SOCK_STREAM, 0);
  if (UNLIKELY(signal = set_socket_nonblocking(sockfd) != 0)) {
    return signal;
  }

  client->ctx.socket = sockfd;

  return 0;
}