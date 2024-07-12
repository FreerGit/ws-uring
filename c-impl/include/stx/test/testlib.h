extern int global_total_tests;
extern int global_failed_tests;

#define ASSERT_TRUE(expression)                                      \
  ++global_total_tests;                                              \
  if (!(expression)) {                                               \
    ++global_failed_tests;                                           \
    printf("%s(%d): expression assert fail.\n", __FILE__, __LINE__); \
  }
