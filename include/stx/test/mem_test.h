
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "base.h"
#include "mem.h"
#include "testlib.h"

extern int global_total_tests;
extern int global_failed_tests;

typedef struct {
  uint64_t a;
  uint64_t b;
  uint64_t c;
  uint64_t d;
} T;

void arena_test(void) {
  size_t        size = 1024 * 64;
  unsigned char backing_buffer[size];
  Arena         arena     = arena_init(backing_buffer, size);
  Allocator     allocator = arena_alloc_init(&arena);

  int      *a = allocator_alloc(int, 100, allocator);
  uint16_t *b = allocator_alloc(uint16_t, 100, allocator);
  size_t   *c = allocator_alloc(size_t, 100, allocator);
  T        *d = allocator_alloc(T, 100, allocator);

  for (int x = 0; x < 100; x++) a[x] = x;

  for (uint16_t y = 0; y < 100; y++) b[y] = y;

  for (size_t z = 0; z < 100; z++) c[z] = z;

  for (uint64_t g = 0; g < 100; g++) d[g].a = g;

  ASSERT_TRUE(a[25] = 25);
  ASSERT_TRUE(b[50] = 50);
  ASSERT_TRUE(c[75] = 75);
  ASSERT_TRUE(d[100].a = 100);

  allocator_free_all(allocator);
  ASSERT_TRUE(((Arena *)allocator.allocator)->offset == 0);
}

void pool_test(void) {
  unsigned char backing_buffer[1024];
  Pool          p;
  uint64_t     *a, *b, *c, *d, *e, *f;
  pool_init(&p, backing_buffer, 1024, 64, DEFAULT_ALIGNMENT);

  a  = (uint64_t *)pool_alloc(&p);
  b  = (uint64_t *)pool_alloc(&p);
  c  = (uint64_t *)pool_alloc(&p);
  d  = (uint64_t *)pool_alloc(&p);
  e  = (uint64_t *)pool_alloc(&p);
  f  = (uint64_t *)pool_alloc(&p);
  *a = 5;
  *b = 5;
  *c = 5;
  *d = 5;
  *e = 5;
  *f = 5;

  pool_free(&p, f);
  pool_free(&p, c);
  pool_free(&p, b);
  pool_free(&p, d);

  d = (uint64_t *)pool_alloc(&p);

  ASSERT_TRUE(*a == 5);
  pool_free(&p, a);

  a = (uint64_t *)pool_alloc(&p);

  pool_free(&p, e);
  pool_free(&p, a);
  pool_free(&p, d);
}