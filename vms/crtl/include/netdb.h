/* netdb.h: host names, from the hosts database or DNS. */
#ifndef __NETDB_LOADED
#define __NETDB_LOADED

#include <decc$types.h>

struct hostent {
	char *h_name;
	char **h_aliases;
	int h_addrtype;
	int h_length;
	char **h_addr_list;
};
#define h_addr h_addr_list[0]

struct hostent *gethostbyname(const char *name) __DECC(gethostbyname);

#endif
