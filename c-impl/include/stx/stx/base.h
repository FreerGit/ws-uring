// Note that you may need to `#define _POSIX_C_SOURCE 200809L` or similar
// because of clock_gettime

#ifndef STX_H
#define STX_H

#include <assert.h>
#include <string.h>
#include <time.h>

/*
  | stdint    | base     |
  |-----------|----------|
  | int8_t    | schar    |
  | uint8_t   | uchar    |
  | int16_t   | short    |
  | uint16_t  | ushort   |
  | int32_t   | int      |
  | uint32_t  | uint     |
  | int64_t   | long     |
  | ptrdiff_t | long     |
  | uint64_t  | ulong    |
  | size_t    | ulong    |
 */

typedef signed char    schar;
typedef unsigned char  uchar;
typedef unsigned short ushort;
typedef unsigned int   uint;
typedef unsigned long  ulong;

/*  This is interesting, can be used in many ways for optimizations.
    Leaving this here because I don't _fully_ understand the implications.*/

// #define assert(c) while (!(c)) __builtin_unreachable()

// Useful macros
#define count_of(a) (size_t)(sizeof(a) / sizeof(*(a)))
#define max(a, b) ((a) > (b) ? (a) : (b))
#define min(a, b) ((a) < (b) ? (a) : (b))
// #define new(a, t, n) (t *)alloc(a, sizeof(t), n)

/* Tell the compiler that the fn does not have any side-effects, this includes
  trivial memory writes. This can be getter fn's, simple constant returns, fn's
  with a single switch case from int -> string (error enums) and so on. */
#define FN_PURE __attribute__((pure))

/* Contract to the compiler that the fn does not depends on the state of memory
 */
#define FN_CONST __attribute__((const))

/* Since the compiler does not always inline, even when you tell it to,
  FN_UNUSED can help when -Winline screams about a static fn. */
#define FN_UNUSED __atrribute__((unused))

/* Hint for the optimizer, evaluates c and returns true/false as long  */
#define LIKELY(c) __builtin_expect(!!(c), 1L)
#define UNLIKELY(c) __builtin_expect(!!(c), 0L)

/*
  switch( return_code ) {
    case RETURN_CASE_1: FALLTHROUGH;
    case RETURN_CASE_2: FALLTHROUGH;
    case RETURN_CASE_3:
      case_123();
    default:
      case_other();
  }
*/
#define FALLTHROUGH       \
  while (0) __attribute__ \
  ((fallthrough))

FN_PURE static inline int memeql(void const *s1, void const *s2, ulong sz) {
  return 0 == memcmp(s1, s2, sz);
}

// Timing helpers

#define get_tickcount() ((long)__builtin_ia32_rdtsc())

#define TICKCOUNT_OF_BLOCK(x)       \
  ({                                \
    long __start = get_tickcount(); \
    x;                              \
    long __end   = get_tickcount(); \
    long __delta = __end - __start; \
    __delta;                        \
  })

#define TIME_A_BLOCK_NS(x)                                  \
  ({                                                        \
    struct timespec __start, __end;                         \
    clock_gettime(CLOCK_MONOTONIC_RAW, &__start);           \
    x;                                                      \
    clock_gettime(CLOCK_MONOTONIC_RAW, &__end);             \
    ulong __delta = (__end.tv_sec - __start.tv_sec) * 1e9 + \
                    (__end.tv_nsec - __start.tv_nsec);      \
    __delta;                                                \
  })

#endif
