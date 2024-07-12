#ifndef URL_H
#define URL_H

#include <arpa/inet.h>

#include "base.h"

/* returns string representation of the error code  */
FN_PURE char *
get_url_error(int error_code);

/*  given a url, produces a ipv4 addr.
    use get_url_error() to get the error as a str */
int
url_to_ipv4(char const *url, char ipv4[INET_ADDRSTRLEN]);

#endif  // URL_H