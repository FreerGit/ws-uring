#include <stdbool.h>
#include <stddef.h>

#include "base.h"
#include "dyn_array.h"
#include "mem.h"

#ifndef STRING_H
#define STRING_H

// ------------------------------------------------------------- //
// A much simpler String type, null-terminated for compatability.
//                    UTF-8 assumed string.
// ------------------------------------------------------------- //

typedef struct {
  char  *data;
  size_t len;
} String8;

#define String8(x)     \
  (String8) {          \
    x, count_of(x) - 1 \
  }

String8 string_init(size_t len, Allocator *a) {
  String8 s = {
      a->alloc(len + 1, a->allocator),
      len,
  };
  s.data[len] = 0;
  return s;
}

String8 string_concat(String8 s1, String8 s2, Allocator *a) {
  size_t  len = s1.len + s2.len;
  String8 s   = string_init(len, a);
  memcpy(s.data, s1.data, s1.len);
  memcpy(&s.data[s1.len], s2.data, s2.len);
  return s;
}

String8 string_substring(String8 s, size_t start, size_t end, Allocator *a) {
  String8 r = {0};
  if (end <= s.len && start < end) {
    r = string_init(end - start, a);
    memcpy(r.data, &s.data[start], r.len);
  }
  return r;
}

bool string_contains(String8 haystack, String8 needle) {
  bool found = false;
  for (size_t i = 0, j = 0; i < haystack.len && !found; i += 1) {
    while (haystack.data[i] == needle.data[j]) {
      j += 1;
      i += 1;
      if (j == needle.len) {
        found = true;
        break;
      }
    }
  }
  return found;
}

size_t string_index_of(String8 haystack, String8 needle) {
  for (size_t i = 0; i < haystack.len; i += 1) {
    size_t j     = 0;
    size_t start = i;
    while (haystack.data[i] == needle.data[j]) {
      j += 1;
      i += 1;
      if (j == needle.len) {
        return start;
      }
    }
  }
  return (size_t)-1;
}

// NOTE: this does not terminate the String8 with a 0 as that would destroy the
// original String8.
String8 string_substring_view(String8 haystack, String8 needle) {
  String8 r           = {0};
  size_t  start_index = string_index_of(haystack, needle);
  if (start_index < haystack.len) {
    r.data = &haystack.data[start_index];
    r.len  = haystack.len - start_index;
  }
  return r;
}

bool string_equal(String8 a, String8 b) {
  if (a.len != b.len) {
    return false;
  }
  return memeql(a.data, b.data, a.len);
}

String8 string_replace(String8 s, String8 match, String8 replacement,
                       Allocator *a) {
  (void)s;
  (void)match;
  (void)replacement;
  (void)a;
  String8 r = {};
  // TODO
  assert(0 && "Unimplemented");
  return r;
}

String8 string_view(String8 s, size_t start, size_t end) {
  if (end < start || end - start > s.len) {
    return (String8){0};
  }
  return (String8){s.data + start, end - start};
}

String8 string_clone(String8 s, Allocator *a) {
  String8 r = {0};
  if (s.len) {
    r.data = a->alloc(s.len, a->allocator);
    r.len  = s.len;
    memcpy(r.data, s.data, s.len);
  }
  return r;
}

String8 *string_split(String8 s, char delimiter, Allocator *a) {
  String8 *arr   = 0;
  size_t   start = 0;
  for (size_t i = 0; i < s.len; i += 1) {
    if (s.data[i] != delimiter) {
      continue;
    } else {
      // Allocate array if we haven't yet.
      if (!arr) {
        arr = array(String8, a);
      }

      // Clone the substring before the delimiter.
      size_t  end    = i;
      String8 cloned = string_substring(s, start, end, a);
      array_append(arr, cloned);
      start = end + 1;
    }
  }
  // Get the last segment.
  if (start + 1 < s.len) {
    String8 cloned = string_substring(s, start, s.len, a);
    array_append(arr, cloned);
  }
  return arr;
}

String8 *string_split_view(String8 s, char delimiter, Allocator *a) {
  String8 *arr   = 0;
  size_t   start = 0;
  for (size_t i = 0; i < s.len; i += 1) {
    if (s.data[i] != delimiter) {
      continue;
    }

    if (!arr) {
      arr = array(String8, a);
    }

    size_t  end  = i;
    String8 view = string_view(s, start, end);
    array_append(arr, view);
    start = end + 1;
  }
  if (start + 1 < s.len) {
    String8 view = string_view(s, start, s.len);
    array_append(arr, view);
  }
  return arr;
}

String8 string_join(String8 *s, char join, Allocator *a) {
  Array_Header *h = array_header(s);

  size_t total_length = 0;
  for (size_t i = 0; i < h->len; i += 1) {
    total_length += s[i].len + 1;  // the length of the string + \0
  }

  char  *mem    = a->alloc(total_length + 1, a->allocator);
  size_t offset = 0;
  for (size_t i = 0; i < h->len; i += 1) {
    memcpy(&mem[offset], s[i].data, s[i].len);
    offset += s[i].len;

    if (i == h->len - 1) {
      break;
    }

    // memcpy(&mem[offset], join, 1);
    mem[offset] = join;
    offset += 1;
  }

  mem[total_length] = 0;

  return (String8){mem, total_length};
}

#endif