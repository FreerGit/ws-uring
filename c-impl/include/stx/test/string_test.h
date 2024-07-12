#include <stdio.h>

#include "base.h"
#include "mem.h"
#include "string8.h"
#include "testlib.h"


void string_test(void) {
  size_t        size = 1024 * 1024;
  unsigned char backing_buffer[size];
  Arena         arena     = arena_init(backing_buffer, size);
  Allocator     allocator = arena_alloc_init(&arena);

  String8 s1 = String8("Hello");
  String8 s2 = String8(" World!");

  String8 cs = string_concat(s1, s2, &allocator);
  ASSERT_TRUE(memeql(cs.data, "Hello World!", cs.len));
  String8 *split = string_split(cs, ' ', &allocator);

  ASSERT_TRUE(memeql(split[0].data, "Hello", split[0].len));
  ASSERT_TRUE(memeql(split[1].data, "World!", split[1].len));

  String8 joined = string_join(split, ' ', &allocator);
  ASSERT_TRUE(memeql(joined.data, "Hello World!", joined.len));

  String8 view = string_substring_view(s1, String8("el"));
  ASSERT_TRUE(memeql(view.data, "ello", view.len));
  ASSERT_TRUE(view.len ==
              count_of("ello") -
                  1);  // One less because the view is _not_ null-terminated

  String8 s8_view_s1 = string_view(s1, 2, 5);
  String8 s8_view_s2 = string_view(s2, 1, 7);

  ASSERT_TRUE(memeql(s8_view_s1.data, "llo", s8_view_s1.len));
  ASSERT_TRUE(s8_view_s1.len ==
              count_of("llo") -
                  1);  // One less because the view is _not_ null-terminated
  ASSERT_TRUE(memeql(s8_view_s2.data, "World!", s8_view_s2.len));
  ASSERT_TRUE(s8_view_s2.len ==
              count_of("World!") -
                  1);  // One less because the view is _not_ null-terminated

  ASSERT_TRUE(string_equal(s1, String8("Hello")));

  String8 a_clone = string_clone(s1, &allocator);
  ASSERT_TRUE(string_equal(s1, String8("Hello")))
  ASSERT_TRUE(string_equal(s1, a_clone));

  allocator_free_all(allocator);
  ASSERT_TRUE(((Arena *)allocator.allocator)->offset == 0);
}