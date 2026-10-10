/*
 * The C run-time library's sockets: a socket is a channel to
 * TCPIP$DEVICE:, driven with $QIOW as TCP/IP Services' socket library
 * drives it (ADR-0024), and a file descriptor, 3 and up, names it.
 * Everything waits: no nonblocking sockets, select or ASTs yet.
 */
#include <descrip.h>
#include <errno.h>
#include <inet.h>
#include <iodef.h>
#include <netdb.h>
#include <socket.h>
#include <ssdef.h>
#include <starlet.h>
#include <stdio.h>
#include <tcpip$inetdef.h>
#include <unistd.h>
#include "crtl.h"

#define FDS 32
/* Each descriptor's channel, 0 if it isn't a socket. 0-2 are the
 * standard streams'. */
static unsigned short chans[FDS];

struct iosb {
	unsigned short status, count;
	unsigned int details;
};

struct item_list_2 {
	unsigned short length, type;
	unsigned int address;	/* 32 bits, as descrip.h says */
};

/* errno for a failed condition value, which vaxc$errno keeps. */
static int fail(int status)
{
	vaxc$errno = status;
	switch (status) {
	case SS$_REJECT: errno = ECONNREFUSED; break;
	case SS$_TIMEOUT: errno = ETIMEDOUT; break;
	case SS$_UNREACHABLE: errno = EHOSTUNREACH; break;
	case SS$_LINKABORT: case SS$_LINKDISCON: errno = ECONNRESET; break;
	case SS$_IVCHAN: errno = EBADF; break;
	case SS$_INSFMEM: errno = ENOMEM; break;
	default: errno = EVMSERR;
	}
	return -1;
}

/* The I/O's status, or its IOSB's. */
static int qiow(unsigned short chan, unsigned func, struct iosb *iosb, void *p1, long p2, void *p3)
{
	int status = sys$qiow(0, chan, func, iosb, 0, 0, p1, p2, p3, 0, 0, 0);

	return status & 1 ? iosb->status : status;
}

static unsigned short chan_of(int s)
{
	return s >= 0 && s < FDS ? chans[s] : 0;
}

int decc$socket_fd(unsigned short chan)
{
	for (int s = 3; s < FDS; s++)
		if (!chans[s]) {
			chans[s] = chan;
			return s;
		}
	errno = EMFILE;
	return -1;
}

unsigned short decc$get_sdc(int s)
{
	return chan_of(s);
}

int socket(int af, int type, int protocol)
{
	$DESCRIPTOR(name, "TCPIP$DEVICE:");
	struct { unsigned short prot; unsigned char type, af; } sockchar;
	struct iosb iosb;
	unsigned short chan;
	int status, s;

	if (af != AF_INET) {
		errno = EAFNOSUPPORT;
		return -1;
	}
	sockchar.prot = protocol ? protocol : type == SOCK_DGRAM ? TCPIP$C_UDP : TCPIP$C_TCP;
	sockchar.type = type;
	sockchar.af = af;
	status = sys$assign(&name, &chan, 0, 0);
	if (!(status & 1))
		return fail(status);
	status = qiow(chan, IO$_SETMODE, &iosb, &sockchar, 0, 0);
	if (!(status & 1) || (s = decc$socket_fd(chan)) < 0) {
		sys$dassgn(chan);
		return status & 1 ? -1 : fail(status);
	}
	return s;
}

int connect(int s, const struct sockaddr *name, socklen_t namelen)
{
	struct item_list_2 item = { namelen, TCPIP$C_SOCK_NAME, (unsigned int)(unsigned long)name };
	struct iosb iosb;
	unsigned short chan = chan_of(s);
	int status;

	if (!chan)
		return fail(SS$_IVCHAN);
	status = qiow(chan, IO$_ACCESS, &iosb, 0, 0, &item);
	return status & 1 ? 0 : fail(status);
}

/* At most 4096 bytes a write (netdriver.mar): send writes that much
 * and says so, as a short write. */
ssize_t send(int s, const void *buf, size_t len, int flags)
{
	struct iosb iosb;
	unsigned short chan = chan_of(s);
	int status;

	(void)flags;
	if (!chan)
		return fail(SS$_IVCHAN);
	if (len > 4096)
		len = 4096;
	status = qiow(chan, IO$_WRITEVBLK, &iosb, (void *)buf, len, 0);
	return status & 1 ? iosb.count : fail(status);
}

/* What came, at most len bytes; 0 once the peer has closed. */
ssize_t recv(int s, void *buf, size_t len, int flags)
{
	struct iosb iosb;
	unsigned short chan = chan_of(s);
	int status;

	(void)flags;
	if (!chan)
		return fail(SS$_IVCHAN);
	if (len > 65535)
		len = 65535;
	status = qiow(chan, IO$_READVBLK, &iosb, buf, len, 0);
	if (status == SS$_LINKDISCON)
		return 0;
	return status & 1 ? iosb.count : fail(status);
}

ssize_t read(int fd, void *buf, size_t n)
{
	return recv(fd, buf, n, 0);
}

ssize_t write(int fd, const void *buf, size_t n)
{
	return send(fd, buf, n, 0);
}

/* Closes the socket and deassigns its channel. */
int close(int s)
{
	struct iosb iosb;
	unsigned short chan = chan_of(s);

	if (!chan)
		return fail(SS$_IVCHAN);
	qiow(chan, IO$_DEACCESS, &iosb, 0, 0, 0);
	chans[s] = 0;
	sys$dassgn(chan);
	return 0;
}

in_addr_t inet_addr(const char *cp)
{
	in_addr_t a = 0;
	unsigned n = 0, parts = 0, digits = 0;

	for (;; cp++) {
		if (*cp >= '0' && *cp <= '9' && n <= 255) {
			n = n * 10 + *cp - '0';
			digits++;
		} else if ((*cp == '.' || !*cp) && digits && n <= 255 && parts < 4) {
			a |= n << 8 * parts++;
			n = digits = 0;
			if (!*cp)
				break;
		} else {
			return (in_addr_t)-1;
		}
	}
	return parts == 4 ? a : (in_addr_t)-1;
}

char *inet_ntoa(struct in_addr in)
{
	static char text[16];
	unsigned char *b = (unsigned char *)&in.s_addr;

	snprintf(text, sizeof text, "%u.%u.%u.%u", b[0], b[1], b[2], b[3]);
	return text;
}

/* The host's address, from the hosts database or DNS: one, no aliases. */
struct hostent *gethostbyname(const char *name)
{
	static struct hostent host;
	static unsigned addr;
	static char *addrs[2] = { (char *)&addr }, *aliases[1];
	static char hname[256];
	int len = 0;

	while (name[len] && len < 255)
		hname[len] = name[len], len++;
	hname[len] = 0;
	if (!(decc$$host_addr(len, hname, &addr) & 1))
		return NULL;
	host.h_name = hname;
	host.h_aliases = aliases;
	host.h_addrtype = AF_INET;
	host.h_length = 4;
	host.h_addr_list = addrs;
	return &host;
}
