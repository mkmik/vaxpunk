/* inet.h: Internet addresses as text. */
#ifndef __INET_LOADED
#define __INET_LOADED

#include <in.h>

in_addr_t inet_addr(const char *cp) __DECC(inet_addr);
char *inet_ntoa(struct in_addr in) __DECC(inet_ntoa);

#endif
