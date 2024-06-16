#include "client.h"
#include "base.h"

FN_PURE char *get_client_error(int error_code) {
  switch (error_code) {
  case 1:
    return "Could not initialize io_uring, kernel version problem?";
  default:
    return "Not an url error, probably using the wrong get_*_error() function";
  }
}

int init_uring(client_t *client, int queue_depth) {
  if (UNLIKELY(io_uring_queue_init(queue_depth, &client->ring, 0) < 0)) {
    return 1;
  }
  return 0;
}