#ifndef LOG_H
#define LOG_H
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#if defined(LOG_WITH_TIME)
#define TIME_EVENT()              \
  ({                              \
    time_t     t = time(NULL);    \
    struct tm *l = localtime(&t); \
    l;                            \
  })

#define log_time()                                                          \
  {                                                                         \
    char       buf[64];                                                     \
    struct tm *event_time                                   = TIME_EVENT(); \
    buf[strftime(buf, sizeof(buf), "%H:%M:%S", event_time)] = '\0';         \
    printf("%s%s\x1b[0m ", "\x1b[90m", buf);                                \
  }
#else
#define log_time() \
  {}
#endif  // LOG_WITH_TIME

#define log(type, color, format, ...)                                         \
  {                                                                           \
    log_time();                                                               \
    printf("%s%s\x1b[0m %s%s:%d:\x1b[0m ", color, type, "\x1b[90m", __FILE__, \
           __LINE__);                                                         \
    printf(format, ##__VA_ARGS__);                                            \
    printf("\n");                                                             \
    fflush(stdout);                                                           \
  }

#if defined(LOG_DEBUG)
#define log_debug(format, ...) log("DEBUG", "\x1b[36m", format, ##__VA_ARGS__)
#else
#define log_debug(format, ...) \
  {}
#endif  // LOG_DEBUG

#define log_info(format, ...) log("INFO ", "\x1b[32m", format, ##__VA_ARGS__)

#define log_error(format, ...) log("ERROR", "\x1b[31m", format, ##__VA_ARGS__)

#define log_fatal(format, ...)                       \
  {                                                  \
    log("FATAL", "\x1b[35m", format, ##__VA_ARGS__); \
    exit(-1);                                        \
  }

#endif
