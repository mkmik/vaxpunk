/* socket.h: sockets, each a channel to TCPIP$DEVICE:, as TCP/IP Services'. */
#ifndef __SOCKET_LOADED
#define __SOCKET_LOADED

#include <decc$types.h>

#define SOCK_STREAM 1
#define SOCK_DGRAM 2

typedef unsigned int socklen_t;
typedef long ssize_t;

struct sockaddr {
	uint16_t sa_family;
	char sa_data[14];
};

int connect(int s, const struct sockaddr *name, socklen_t namelen) __DECC(connect);
ssize_t recv(int s, void *buf, size_t len, int flags) __DECC(recv);
ssize_t send(int s, const void *buf, size_t len, int flags) __DECC(send);
int socket(int af, int type, int protocol) __DECC(socket);
/* The socket whose channel is chan, as a file descriptor. */
int decc$socket_fd(unsigned short chan) __DECC(socket_fd);
/* And a socket's channel. */
unsigned short decc$get_sdc(int s) __DECC(get_sdc);

#endif
