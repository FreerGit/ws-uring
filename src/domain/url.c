#include "url.h"
#include "base.h"
#include "log.h"

#include <arpa/inet.h>
#include <netdb.h>
#include <netinet/in.h>
#include <string.h>

FN_PURE char *get_url_error(int error_code) {
  switch (error_code) {
  case 1:
    return "Could not resolve hostname";
  default:
    return "Not an url error, probably using the wrong get_*_error() function";
  }
}

int url_to_ipv4(char const *url, char ipv4[INET_ADDRSTRLEN]) {
  struct hostent *host_info = gethostbyname(url);
  if (UNLIKELY(host_info == NULL)) {
    return 1;
  }

  if (LIKELY(host_info->h_addrtype == AF_INET &&
             host_info->h_addr_list[0] != NULL)) {
    struct in_addr *ipv4_addr = (struct in_addr *)host_info->h_addr_list[0];
    char ip_buffer[INET_ADDRSTRLEN];
    const char *ipv4_str =
        inet_ntop(AF_INET, ipv4_addr, ip_buffer, INET_ADDRSTRLEN);

    if (LIKELY(ipv4_str != NULL)) {
      strcpy(ipv4, ipv4_str);
      return 0;
    } else {
      return 1;
    }
  } else {
    return 1;
  }
}
