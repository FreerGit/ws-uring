#define _POSIX_C_SOURCE 200809L
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#include "base.h"
#include "dyn_array.h"
#include "dyn_array_test.h"
#include "mem.h"
#include "mem_test.h"
#include "string_test.h"
#include "testlib.h"


#define LOG_DEBUG
#define LOG_WITH_TIME
#include "log.h"

int global_total_tests;
int global_failed_tests;

int main() {
  int result = (global_failed_tests != 0);

  ulong ns;
  ns = TIME_A_BLOCK_NS(pool_test());
  log_info("Pool test took: %ld", ns);

  ns = TIME_A_BLOCK_NS(arena_test());
  log_info("Arean test took: %ld", ns);

  ns = TIME_A_BLOCK_NS(string_test());
  log_info("String8 test took: %ld", ns);

  ns = TIME_A_BLOCK_NS(dyn_array_test());
  log_info("Dynamic array test took: %ld", ns);

  printf("%s: %d/%d passed.\e[0m\n",
         result ? "\x1B[31mUnit Tests Failed" : "\x1B[32mUnit Tests Successful",
         global_total_tests - global_failed_tests, global_total_tests);
}
