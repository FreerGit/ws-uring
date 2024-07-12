#include <wolfssl/wolfio.h>
#define _POSIX_C_SOURCE 200809L

#define LOG_DEBUG
#define LOG_WITH_TIME
#include <arpa/inet.h>
#include <bits/types/sigset_t.h>
#include <fcntl.h>
#include <liburing.h>
#include <netdb.h>
#include <netinet/in.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>
#include <wolfssl/options.h>
#include <wolfssl/ssl.h>

#include "base.h"
#include "client.h"
#include "log.h"

#define QUEUE_DEPTH 64
#define MAX_BUFFER_SIZE 4096 * 2

void
prep_read(int fd, struct io_uring *ring, size_t max_buff_size) {
  log_debug("%p", ring);
  struct io_uring_sqe *sqe = io_uring_get_sqe(ring);
  if (!sqe) {
    log_fatal("could not get sqe");
  }

  struct iovec *req = malloc(sizeof(struct iovec));
  req->iov_base     = malloc(max_buff_size);
  req->iov_len      = max_buff_size;

  memcpy(&sqe->user_data, &req, sizeof(req));
  io_uring_prep_readv(sqe, fd, req, 1, 0);
  io_uring_sqe_set_data(sqe, req);
  io_uring_submit(ring);
}

void
prep_send(int fd, struct io_uring *ring, char *buf, size_t sz) {
  log_debug("prep_send");
  struct io_uring_sqe *sqe = io_uring_get_sqe(ring);
  io_uring_prep_send(sqe, fd, buf, sz, 0);
  io_uring_submit(ring);
}

bool to_prep = true; /* TODO */

int
CbIORecv(WOLFSSL *ssl, char *buf, int sz, void *ctx) {
  (void)ssl;
  struct io_uring_ctx *cc = (io_uring_ctx *)ctx;

  int ret = 0;
  if (to_prep) {
    log_debug("%p", &cc->ring);
    prep_read(cc->socket, &cc->ring, sz);
  }
  // log_info("called");
  int ret_ret;
  ret_ret = io_uring_peek_cqe(&cc->ring, &cc->cqe);

  if (ret_ret != -EAGAIN) {
    struct iovec *data = (struct iovec *)cc->cqe->user_data;
    memcpy(buf, data->iov_base, cc->cqe->res);
    ret = cc->cqe->res;
    sz  = cc->cqe->res;
    io_uring_cqe_seen(&cc->ring, cc->cqe);
    to_prep = true;
  } else {
    ret     = WOLFSSL_CBIO_ERR_WANT_READ;
    to_prep = false;
  }

  return ret;
}

int
CbIOSend(WOLFSSL *ssl, char *buf, int sz, void *ctx) {
  (void)ssl;
  struct io_uring_ctx *cc = (io_uring_ctx *)ctx;
  log_debug("%d", cc->socket);

  int sent;
  prep_send(cc->socket, &cc->ring, buf, sz);
  int ret_ret;
  ret_ret = io_uring_peek_cqe(&cc->ring, &cc->cqe);

  if (ret_ret != -EAGAIN) {
    sent = cc->cqe->res;
    io_uring_cqe_seen(&cc->ring, cc->cqe);
  } else {
    sent = 0;
  }

  return sent;
}

int
main() {
  struct client_t client = {};
  int             signal = client_init(&client, 1024 * 8);
  if (UNLIKELY(signal != 0)) {
    log_error("%s", get_client_error(signal));
  }
  // Initialize WolfSSL
  wolfSSL_Init();

  // Create a WolfSSL context
  WOLFSSL_CTX *ctx = wolfSSL_CTX_new(wolfSSLv23_client_method());
  if (ctx == NULL) {
    fprintf(stderr, "Failed to create WolfSSL context\n");
    return 1;
  }

  wolfSSL_SetIORecv(ctx, CbIORecv);
  wolfSSL_SetIOSend(ctx, CbIOSend);

  struct sockaddr_in server_addr;
  memset(&server_addr, 0, sizeof(server_addr));
  server_addr.sin_family = AF_INET;
  server_addr.sin_port   = htons(443);  // HTTPS port
  if (inet_pton(AF_INET, "151.101.2.137", &server_addr.sin_addr) <=
      0) {  // www.example.com IP
    fprintf(stderr, "Invalid address\n");
    return 1;
  }

  // Prepare the connect operation
  struct io_uring_sqe *sqe = io_uring_get_sqe(&client.ctx.ring);
  if (!sqe) {
    log_error("io_uring_get_sqe: queue is full\n");
    io_uring_queue_exit(&client.ctx.ring);
    close(client.ctx.socket);
    return 1;
  }
  io_uring_prep_connect(sqe, client.ctx.socket, (struct sockaddr *)&server_addr,
                        sizeof(server_addr));

  // Submit the request
  int conn_ret = io_uring_submit(&client.ctx.ring);
  if (conn_ret < 0) {
    log_error("io_uring_submit: %d\n", -conn_ret);
    io_uring_queue_exit(&client.ctx.ring);
    close(client.ctx.socket);
    return 1;
  }

  ulong ns = TIME_A_BLOCK_NS({
    // Poll for completion
    while (1) {
      conn_ret = io_uring_peek_cqe(&client.ctx.ring, &client.ctx.cqe);
      if (conn_ret == -EAGAIN) {
        // No completion yet, continue polling
        continue;
      } else if (conn_ret < 0) {
        fprintf(stderr, "io_uring_peek_cqe: %s\n", strerror(-conn_ret));
        io_uring_queue_exit(&client.ctx.ring);
        close(client.ctx.socket);
        return 1;
      } else {
        break;
      }
    }
  });

  // Process the completion
  if (client.ctx.cqe->res < 0) {
    fprintf(stderr, "Async connect failed: %s\n",
            strerror(-client.ctx.cqe->res));
    io_uring_queue_exit(&client.ctx.ring);
    close(client.ctx.socket);
    return 1;
  }
  log_info("Socket connection took %ld ns", ns);

  io_uring_cqe_seen(&client.ctx.ring, client.ctx.cqe);

  // Create a WolfSSL object
  wolfSSL_CTX_set_verify(ctx, WOLFSSL_VERIFY_NONE, NULL);
  WOLFSSL *ssl = wolfSSL_new(ctx);
  if (ssl == NULL) {
    fprintf(stderr, "Failed to create WolfSSL object\n");
    return 1;
  }

  //   wolfSSL_set_fd(ssl, sockfd);
  wolfSSL_SetIOReadCtx(ssl, &client.ctx);
  wolfSSL_SetIOWriteCtx(ssl, &client.ctx);
  // TODO
  int ret;

  ns = TIME_A_BLOCK_NS({
    while ((ret = wolfSSL_connect(ssl)) != SSL_SUCCESS) {
      int error = wolfSSL_get_error(ssl, ret);

      if (error == SSL_ERROR_WANT_READ || error == SSL_ERROR_WANT_WRITE) {
        // Busy-polling: keep retrying
        continue;
      } else {
        // Handle other errors (e.g., SSL handshake failure)
        printf("wolfSSL_connect error: %d\n", error);
        char errorString[80];
        int  err_c = wolfSSL_get_error(ssl, ret);
        log_error("%d", err_c);
        wolfSSL_ERR_error_string(err_c, errorString);
        log_error("%s", errorString);
        wolfSSL_free(ssl);
        wolfSSL_CTX_free(ctx);
        wolfSSL_Cleanup();
        close(client.ctx.socket);
        return -1;
      }
    }
    // Perform the TLS/SSL handshake
  });
  log_debug("TLS connect took %ld ns", ns);
  // Allocate buffers for read and write operations
  char read_buffer[MAX_BUFFER_SIZE];
  char write_buffer[] = "GET / HTTP/1.1\r\nHost:www.wolfssl.com\r\n\r\n";

  ns = TIME_A_BLOCK_NS({
    if ((ret = wolfSSL_write(ssl, write_buffer, strlen(write_buffer))) !=
        strlen(write_buffer)) {
      fprintf(stderr, "ERROR: failed to write\n");
      // goto exit;
    }
  });
  log_debug("send took %ld ns", ns);

  int  r;
  char buff[MAX_BUFFER_SIZE];
  memset(buff, 0, sizeof(buff));

  ns = TIME_A_BLOCK_NS({
    while ((ret = wolfSSL_read(ssl, buff, sizeof(buff) - 1)) < 0) {
      int error = wolfSSL_get_error(ssl, ret);

      if (error == SSL_ERROR_WANT_READ || error == SSL_ERROR_WANT_WRITE) {
        // Busy-polling: keep retrying
        continue;
      } else {
        // Handle other errors
        printf("wolfSSL_read error: %d\n", error);
        return -1;  // Return error
      }
    }
  });

  log_info("read in %ld ns, \n%s", ns, buff);

  io_uring_queue_exit(&client.ctx.ring);
  wolfSSL_free(ssl);
  wolfSSL_CTX_free(ctx);
  wolfSSL_Cleanup();

  return 0;
}