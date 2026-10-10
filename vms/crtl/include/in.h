/* in.h: Internet addresses. */
#ifndef __IN_LOADED
#define __IN_LOADED

#include <decc$types.h>

#define AF_INET 2
#define INADDR_ANY 0u
#define IPPROTO_TCP 6
#define IPPROTO_UDP 17

typedef uint32_t in_addr_t;
typedef uint16_t in_port_t;

struct in_addr {
	in_addr_t s_addr;	/* network order */
};

/* BSD 4.3's, as DEC C's by default: no length. */
struct sockaddr_in {
	uint16_t sin_family;
	in_port_t sin_port;	/* network order */
	struct in_addr sin_addr;
	char sin_zero[8];
};

#define htons(x) ((uint16_t)__builtin_bswap16(x))
#define ntohs(x) ((uint16_t)__builtin_bswap16(x))
#define htonl(x) ((uint32_t)__builtin_bswap32(x))
#define ntohl(x) ((uint32_t)__builtin_bswap32(x))

#endif
