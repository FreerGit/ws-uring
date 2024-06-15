#include <stdio.h>

#include "base.h"
#include "dyn_array.h"
#include "mem.h"
#include "string8.h"
#include "testlib.h"

typedef struct {
  String8 name;
  ulong   age;
  ulong   num1;
  ulong   num2;
  ulong   num3;
  ulong   num4;
  ulong   num5;
} Large;

// static_assert(sizeof(Large) == 64);

bool assert_struct_values(Large l) {
  return string_equal(l.name, String8("Large")) && l.age == 0 && l.num1 == 1 &&
         l.num2 == 2 && l.num3 == 3 && l.num3 == 3 && l.num4 == 4 &&
         l.num5 == 5;
}

void dyn_array_test(void) {
  size_t        size = 1024 * 1024;
  unsigned char backing_buffer[size];
  Arena         arena     = arena_init(backing_buffer, size);
  Allocator     allocator = arena_alloc_init(&arena);

  Large *arr = array(Large, &allocator);

  Large one = {String8("Large"), 0, 1, 2, 3, 4, 5};

  array_append(arr, one);
  ASSERT_TRUE(string_equal(arr[0].name, String8("Large")));
  ASSERT_TRUE(array_length(arr) == 1);

  array_append(arr, one);
  ASSERT_TRUE(string_equal(arr[1].name, String8("Large")));
  ASSERT_TRUE(array_length(arr) == 2);

  array_append(arr, one);
  array_append(arr, one);
  ASSERT_TRUE(string_equal(arr[2].name, String8("Large")));
  ASSERT_TRUE(string_equal(arr[3].name, String8("Large")));
  ASSERT_TRUE(array_length(arr) == 4);

  allocator_free_all(allocator);
  Large *new_arr = array(Large, &allocator);

  for (size_t i = 0; i < 100; i++) {
    array_append(new_arr, one);
  }

  for (size_t i = 0; i < 100; i++) {
    ASSERT_TRUE(assert_struct_values(new_arr[i]));
  }

  ASSERT_TRUE(array_length(new_arr) == 100);
  allocator_free_all(allocator);
}
