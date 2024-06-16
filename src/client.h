#ifndef CLIENT_H
#define CLIENT_H

#include <liburing.h>

#include "base.h"

/* A note on the submission queue, the sqe is only valid until io_uring_submit()
   is called therefore a pointer to the sqe within io_uring_ctx would be
   invalidated, call io_uring_get_sqe() instead.
*/
typedef struct io_uring_ctx {
  struct io_uring_cqe *cqe; /* completion queue */
  struct io_uring      ring;
  int                  socket;
} io_uring_ctx;

typedef struct client_t {
  struct io_uring_ctx ctx; /* context for the IO callbacks */
} client_t;

/* returns string representation of the error code  */
FN_PURE char *
get_client_error(int error_code);

int
client_init(client_t *client, int queue_depth);

#endif  // !CLIENT_H