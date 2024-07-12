#include "base.h"

#ifndef MEM_H
#define MEM_H

#include <assert.h>
#include <stdbool.h>
#include <stddef.h>

// ------------------------------------------------------------- //
//                          Helpers
// ------------------------------------------------------------- //

bool is_power_of_two(long x) {
  return (x & (x - 1)) == 0;
}

size_t align_forward_size(size_t ptr, size_t align) {
  size_t p, a, modulo;

  assert(is_power_of_two((long)align));

  p = ptr;
  a = (long)align;
  // Same as (p % a) but faster as 'a' is a power of two
  modulo = p & (a - 1);

  if (modulo != 0) {
    // If 'p' address is not aligned, push the address to the
    // next value which is aligned
    p += a - modulo;
  }
  return p;
}

long align_forward_uintptr(long ptr, long align) {
  long a, p, modulo;

  assert(is_power_of_two(align));

  a      = align;
  p      = ptr;
  modulo = p & (a - 1);
  if (modulo != 0) {
    p += a - modulo;
  }
  return p;
}

// ------------------------------------------------------------- //
//                       Allocator Interface
// ------------------------------------------------------------- //

typedef struct {
  void *(*alloc)(size_t bytes, void *allocator);
  void *(*alloc_align)(size_t bytes, size_t align, void *allocator);
  void *(*resize)(void *old_mem, size_t old_size, size_t new_size,
                  void *allocator);
  void *(*resize_align)(void *old_mem, size_t old_size, size_t new_size,
                        size_t align, void *allocator);
  void (*free)(size_t bytes, void *ptr, void *allocator);
  void (*free_all)(void *allocator);
  void *allocator;
} Allocator;

#define allocator_alloc(T, count, a) \
  ((T *)((a).alloc(sizeof(T) * count, (a).allocator)))
#define allocator_alloc_align(T, count, align, a) \
  ((T *)((a)->alloc_align(sizeof(T) * count, align, (a)->allocator)))
#define allocator_resize(old_mem, old_size, new_size, a) \
  { (a)->resize(old_mem, old_size, new_size, (a)->allocator) }
#define allocator_resize_align(old_mem, old_size, new_size, align, a) \
  { (a)->resize_align(old_mem, old_size, new_size, align, (a)->allocator) }
#define allocator_free(bytes, ptr, a) ((a).free(bytes, ptr, (a)allocator))
#define allocator_free_all(a) ((a).free_all((a).allocator))

// ------------------------------------------------------------- //
//                  Arena Allocator (Linear Allactor)
// ------------------------------------------------------------- //

#define DEFAULT_ALIGNMENT (2 * sizeof(void *))

typedef struct {
  unsigned char *base;
  size_t         size;
  size_t         offset;
  size_t         committed;
} Arena;

#define arena_alloc_init(a)                                             \
  (Allocator) {                                                         \
    arena_alloc, arena_alloc_aligned, arena_resize, arena_resize_align, \
        arena_free, arena_free_all, a                                   \
  }

#define is_power_of_two(x) ((x != 0) && ((x & (x - 1)) == 0))

long align_forward(long ptr, size_t alignment) {
  long p, a, modulo;
  if (!is_power_of_two(alignment)) {
    return 0;
  }

  p      = ptr;
  a      = (long)alignment;
  modulo = p & (a - 1);

  if (modulo) {
    p += a - modulo;
  }

  return p;
}

void *arena_alloc_aligned(size_t size, size_t alignment, void *a) {
  Arena *arena    = (Arena *)a;
  long   curr_ptr = (long)arena->base + (long)arena->offset;
  long   offset   = align_forward(curr_ptr, alignment);
  offset -= (long)arena->base;

  if (offset + size > arena->size) {
    return 0;
  }

  arena->committed += size;
  void *ptr     = (uchar *)arena->base + offset;
  arena->offset = offset + size;

  return ptr;
}

void *arena_alloc(size_t size, void *allocator) {
  if (!size) {
    return 0;
  }
  return arena_alloc_aligned(size, DEFAULT_ALIGNMENT, (Arena *)allocator);
}

// Does nothing.
void arena_free(size_t size, void *ptr, void *allocator) {
  (void)ptr;
  (void)size;
  (void)allocator;
}

void arena_free_all(void *allocator) {
  Arena *a     = allocator;
  a->offset    = 0;
  a->committed = 0;
}

Arena arena_init(void *buffer, size_t size) {
  return (Arena){.base = buffer, .size = size};
}

void *arena_resize_align(void *old_memory, size_t old_size, size_t new_size,
                         size_t align, void *a) {
  unsigned char *old_mem = (unsigned char *)old_memory;
  Arena         *arena   = (Arena *)a;

  assert(is_power_of_two(align));

  if (old_mem == NULL || old_size == 0) {
    return arena_alloc_aligned(new_size, align, a);
  } else if (arena->base <= old_mem && old_mem < arena->base + arena->size) {
    if (arena->base + arena->committed == old_mem) {
      arena->offset = arena->committed + new_size;
      if (new_size > old_size) {
        // ZII
        memset(&arena->base[arena->offset], 0, new_size - old_size);
      }
      return old_memory;
    } else {
      void  *new_memory = arena_alloc_aligned(new_size, align, a);
      size_t copy_size  = old_size < new_size ? old_size : new_size;
      // Copy across old memory to the new memory
      memcpy(new_memory, old_memory, copy_size);
      return new_memory;
    }

  } else {
    assert(0 && "Memory is out of bounds of the buffer in this arena");
    return NULL;
  }
}

// Because C doesn't have default parameters
void *arena_resize(void *old_memory, size_t old_size, size_t new_size,
                   void *allocator) {
  return arena_resize_align(old_memory, old_size, new_size, DEFAULT_ALIGNMENT,
                            (Arena *)allocator);
}

// // Does nothing.
// void arena_free(size_t bytes, void *allocator, void *ptr) {
//   (void)bytes;
//   (void)allocator;
//   (void)ptr;
// }

// void arena_free_all(void *allocator) {
//   Arena *a = allocator;
//   a->curr_offset = 0;
//   a->prev_offset = 0;
// }

// TODO(imp) sub-arena

// typedef struct {
//   Arena *arena;
//   size_t prev_offset;
//   size_t curr_offset;
// } Temp_Arena_Memory;

// Temp_Arena_Memory temp_arena_memory_begin(Arena *a) {
//   return (Temp_Arena_Memory){.prev_offset = a->prev_offset, .curr_offset =
//   a->curr_offset};
// }

// void temp_arena_memory_end(Temp_Arena_Memory temp) {
//   temp.arena->prev_offset = temp.prev_offset;
//   temp.arena->curr_offset = temp.curr_offset;
// }

// ------------------------------------------------------------- //
//                Pool Allocator (Block Allocator)
// ------------------------------------------------------------- //

typedef struct Pool_Free_Node Pool_Free_Node;
struct Pool_Free_Node {
  Pool_Free_Node *next;
};

typedef struct {
  unsigned char *buf;
  size_t         buf_len;
  size_t         chunk_size;

  Pool_Free_Node *head;  // Free List Head
} Pool;

void pool_free_all(Pool *p);

void pool_init(Pool *p, void *backing_buffer, size_t backing_buffer_length,
               size_t chunk_size, size_t chunk_alignment) {
  // Align backing buffer to the specified chunk alignment
  long initial_start = (long)backing_buffer;
  long start = align_forward_uintptr(initial_start, (long)chunk_alignment);
  backing_buffer_length -= (size_t)(start - initial_start);

  // Align chunk size up to the required chunk_alignment
  chunk_size = align_forward_size(chunk_size, chunk_alignment);

  // Assert that the parameters passed are valid
  assert(chunk_size >= sizeof(Pool_Free_Node) && "Chunk size is too small");
  assert(backing_buffer_length >= chunk_size &&
         "Backing buffer length is smaller than the chunk size");

  // Store the adjusted parameters
  p->buf        = (unsigned char *)backing_buffer;
  p->buf_len    = backing_buffer_length;
  p->chunk_size = chunk_size;
  p->head       = NULL;  // Free List Head

  // Set up the free list for free chunks
  pool_free_all(p);
}

void *pool_alloc(Pool *p) {
  // Get latest free node
  Pool_Free_Node *node = p->head;

  if (node == NULL) {
    assert(0 && "Pool allocator has no free memory");
    return NULL;
  }

  // Pop free node
  p->head = p->head->next;

  // Zero memory by default
  return memset(node, 0, p->chunk_size);
}

void pool_free(Pool *p, void *ptr) {
  Pool_Free_Node *node;

  void *start = p->buf;
  void *end   = &p->buf[p->buf_len];

  if (ptr == NULL) {
    // Ignore NULL pointers
    return;
  }

  if (!(start <= ptr && ptr < end)) {
    assert(0 && "Memory is out of bounds of the buffer in this pool");
    return;
  }

  // Push free node
  node       = (Pool_Free_Node *)ptr;
  node->next = p->head;
  p->head    = node;
}

void pool_free_all(Pool *p) {
  size_t chunk_count = p->buf_len / p->chunk_size;
  size_t i;

  // Set all chunks to be free
  for (i = 0; i < chunk_count; i++) {
    void           *ptr  = &p->buf[i * p->chunk_size];
    Pool_Free_Node *node = (Pool_Free_Node *)ptr;
    // Push free node onto thte free list
    node->next = p->head;
    p->head    = node;
  }
}

#endif