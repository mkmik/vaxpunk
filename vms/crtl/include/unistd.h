/* unistd.h: file descriptors. */
#ifndef __UNISTD_LOADED
#define __UNISTD_LOADED

#include <socket.h>

int close(int fd) __DECC(close);
ssize_t read(int fd, void *buf, size_t n) __DECC(read);
ssize_t write(int fd, const void *buf, size_t n) __DECC(write);

#endif
