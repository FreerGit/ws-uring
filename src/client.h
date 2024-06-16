#ifndef CLIENT_H
#define CLIENT_H

#include "base.h"
#include <liburing.h>

typedef struct client_t {
  struct io_uring ring;
} client_t;

/* returns string representation of the error code  */
FN_PURE char *get_client_error(int error_code);

int init_uring(client_t *client, int queue_depth);

#endif // !CLIENT_H