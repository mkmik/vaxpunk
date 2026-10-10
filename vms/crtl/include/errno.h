/* errno.h: the error numbers, DEC C's values. */
#ifndef __ERRNO_LOADED
#define __ERRNO_LOADED

#include <decc$types.h>

extern int errno __DECC(errno);
/* The condition value behind EVMSERR. */
extern int vaxc$errno;

#define EPERM 1
#define ENOENT 2
#define EIO 5
#define EBADF 9
#define EAGAIN 11
#define ENOMEM 12
#define EACCES 13
#define EFAULT 14
#define EINVAL 22
#define EMFILE 24
#define ENOSPC 28
#define EPIPE 32
#define ERANGE 34
#define EWOULDBLOCK 35
#define ENOTSOCK 38
#define EAFNOSUPPORT 47
#define ENETUNREACH 51
#define ECONNRESET 54
#define ENOTCONN 57
#define ETIMEDOUT 60
#define ECONNREFUSED 61
#define EHOSTUNREACH 65
#define EVMSERR 65535

#endif
