/*
 * The C run-time library's memory, the program's start and end, and its
 * errors.
 */
#include <assert.h>
#include <errno.h>
#include <starlet.h>
#include <stdlib.h>
#include <string.h>
#include "crtl.h"

int errno, vaxc$errno;

/*
 * Memory: a first-fit list of free blocks in address order, each after a
 * header that says its size in units, merged with its neighbours when
 * freed, as Kernighan and Ritchie's. More comes from $EXPREG, at least
 * CHUNK pages at a time.
 */
#define CHUNK 16
typedef struct header {
	struct {
		struct header *next;
		size_t units;
	} h;
} __attribute__((aligned(16))) Header;

static Header base, *freep;

void free(void *p)
{
	Header *b, *q;

	if (!p)
		return;
	b = (Header *)p - 1;
	for (q = freep; !(b > q && b < q->h.next); q = q->h.next)
		if (q >= q->h.next && (b > q || b < q->h.next))
			break;
	if (b + b->h.units == q->h.next) {
		b->h.units += q->h.next->h.units;
		b->h.next = q->h.next->h.next;
	} else {
		b->h.next = q->h.next;
	}
	if (q + q->h.units == b) {
		q->h.units += b->h.units;
		q->h.next = b->h.next;
	} else {
		q->h.next = b;
	}
	freep = q;
}

static Header *more(size_t units)
{
	size_t pages = (units * sizeof(Header) + 4095) / 4096;
	Header *b;

	if (pages < CHUNK)
		pages = CHUNK;
	b = decc$$expreg(pages);
	if (!b)
		return NULL;
	b->h.units = pages * 4096 / sizeof(Header);
	free(b + 1);
	return freep;
}

void *malloc(size_t size)
{
	size_t units = (size + sizeof(Header) - 1) / sizeof(Header) + 1;
	Header *p, *prev;

	if (!freep) {
		base.h.next = freep = &base;
		base.h.units = 0;
	}
	for (prev = freep, p = prev->h.next;; prev = p, p = p->h.next) {
		if (p->h.units >= units) {
			if (p->h.units == units) {
				prev->h.next = p->h.next;
			} else {
				p->h.units -= units;
				p += p->h.units;
				p->h.units = units;
			}
			freep = prev;
			return p + 1;
		}
		if (p == freep && !(p = more(units))) {
			errno = ENOMEM;
			return NULL;
		}
	}
}

void *calloc(size_t n, size_t size)
{
	void *p;

	if (size && n > (size_t)-1 / size) {
		errno = ENOMEM;
		return NULL;
	}
	p = malloc(n * size);
	if (p)
		memset(p, 0, n * size);
	return p;
}

void *realloc(void *p, size_t size)
{
	size_t old;
	void *q;

	if (!p)
		return malloc(size);
	old = (((Header *)p - 1)->h.units - 1) * sizeof(Header);
	if (size <= old)
		return p;
	q = malloc(size);
	if (q) {
		memcpy(q, p, old);
		free(p);
	}
	return q;
}

char *getenv(const char *name)
{
	static char value[256];
	unsigned len;

	if (!(decc$$trnlnm(name, strlen(name), value, sizeof value - 1, &len) & 1))
		return NULL;
	value[len] = 0;
	return value;
}

/* exit(0) ends the image with SS$_NORMAL, any other status with itself,
 * a condition value, as DEC C's exit does. */
void exit(int status)
{
	decc$$flush_all();
	sys$exit(status == 0 ? SS$_NORMAL : status);
	for (;;)
		;
}

void abort(void)
{
	exit(SS$_ABORT);
}

void __assert(const char *e, const char *file, int line)
{
	fprintf(stderr, "Assertion failed: %s, file %s, line %d\n", e, file, line);
	abort();
}

char *strerror(int e)
{
	static const struct {
		int e;
		const char *text;
	} texts[] = {
		{ EPERM, "not owner" },
		{ ENOENT, "no such file or directory" },
		{ EIO, "I/O error" },
		{ EBADF, "bad file number" },
		{ EAGAIN, "no more processes" },
		{ ENOMEM, "not enough core" },
		{ EACCES, "permission denied" },
		{ EFAULT, "bad address" },
		{ EINVAL, "invalid argument" },
		{ EMFILE, "too many open files" },
		{ ENOSPC, "no space left on device" },
		{ EPIPE, "broken pipe" },
		{ ERANGE, "result too large" },
		{ EWOULDBLOCK, "operation would block" },
		{ ENOTSOCK, "socket operation on non-socket" },
		{ EAFNOSUPPORT, "address family not supported" },
		{ ENETUNREACH, "network is unreachable" },
		{ ECONNRESET, "connection reset by peer" },
		{ ENOTCONN, "socket is not connected" },
		{ ETIMEDOUT, "connection timed out" },
		{ ECONNREFUSED, "connection refused" },
		{ EHOSTUNREACH, "no route to host" },
		{ EVMSERR, "non-translatable vms error code" },
	};

	/* EVMSERR's is vaxc$errno's message, as DEC C's. */
	static char message[256];
	unsigned len;

	if (e == EVMSERR && decc$$getmsg(vaxc$errno, message, sizeof message - 1, &len) & 1) {
		message[len] = 0;
		return message;
	}
	for (size_t k = 0; k < sizeof texts / sizeof texts[0]; k++)
		if (texts[k].e == e)
			return (char *)texts[k].text;
	return "unknown error";
}
