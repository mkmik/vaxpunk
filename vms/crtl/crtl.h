/* What the C run-time library's modules share: decc$vms.mar's routines,
 * which VMS's side of each goes through, and the streams' layout. */
#ifndef CRTL_H
#define CRTL_H

#include <ssdef.h>
#include <stdio.h>

void *decc$$expreg(unsigned pages);
int decc$$put(const char *text, unsigned len);
int decc$$get(char *buf, unsigned size, const char *prompt, unsigned plen, unsigned *len);
int decc$$open(void *block, const char *name, unsigned len);
int decc$$read(void *block, char *buf, unsigned size, unsigned *len);
int decc$$close(void *block);
int decc$$foreign(char *buf, unsigned size, unsigned *len);
int decc$$getmsg(unsigned msgid, char *buf, unsigned size, unsigned *len);
int decc$$trnlnm(const char *name, unsigned nlen, char *buf, unsigned size, unsigned *len);
int decc$$host_addr(unsigned len, const char *name, unsigned *addr);

/* A stream: SYS$OUTPUT's, written a record a line; SYS$INPUT's, read a
 * record a line; or a file's, read through RMS. A record read is a line,
 * with a line feed after it. */
#define STREAM_MAX 1024
struct _iobuf {
	enum { CLOSED, OUTPUT, INPUT, FILE_READ } kind;
	int eof, err;
	unsigned len, at;	/* the bytes in buf, and the next one */
	char buf[STREAM_MAX + 1];
	char rms[256];		/* a file's FAB and RAB: decc$$open */
};

void decc$$flush_all(void);

#endif
