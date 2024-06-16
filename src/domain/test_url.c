#define LOG_DEBUG
#define LOG_WITH_TIME
#include "base.h"
#include "log.h"
#include "url.h"

int global_total_tests;
int global_failed_tests;

#define ASSERT_TRUE(expression)                                                \
  ++global_total_tests;                                                        \
  if (!(expression)) {                                                         \
    ++global_failed_tests;                                                     \
    log_error("From function %s", __func__);                                   \
  }

void convert_url() {
  char ipv4[100]; // Should be 16, just in this test case
  int ret = 0;
  if (UNLIKELY((ret = url_to_ipv4("www.google.com", ipv4)) != 0)) {
    log_fatal("%s", get_url_error(ret));
  }

  ASSERT_TRUE(ret == 0);
  int ip_len = strlen(ipv4);
  ASSERT_TRUE(ip_len >= 9 && ip_len <= 16);

  ret = url_to_ipv4("www.invalid-url", ipv4);
  ASSERT_TRUE(get_url_error(ret) == get_url_error(1));
  ret = url_to_ipv4("invalid/url.com", ipv4);
  ASSERT_TRUE(get_url_error(ret) == get_url_error(1));
}

int main() {

  convert_url();
  int result = (global_failed_tests != 0);

  if (UNLIKELY(global_failed_tests != 0)) {
    log_fatal("%s: %d/%d passed.\e[0m\n",
              result ? "\x1B[31mOnly Some Tests Passed"
                     : "\x1B[32mUnit Tests Successful",
              global_total_tests - global_failed_tests, global_total_tests);
  } else {
    log_info("%s: %d/%d passed.\e[0m\n",
             result ? "\x1B[31mUnit Tests Failed"
                    : "\x1B[32mUnit Tests Successful",
             global_total_tests - global_failed_tests, global_total_tests);
  }
}