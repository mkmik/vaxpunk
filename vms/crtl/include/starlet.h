/* starlet.h: the system services C calls. A call goes through a jacket
 * in decc$vms.mar, which gives the service its argument count, as a C
 * compiler that knows the calling standard would (DESIGN-0004). */
#ifndef __STARLET_LOADED
#define __STARLET_LOADED

#include <decc$types.h>

struct dsc$descriptor_s;

int sys$assign(void *devnam, unsigned short *chan, unsigned int acmode, void *mbxnam) __asm__("decc$$sys_assign");
int sys$dassgn(unsigned short chan) __asm__("decc$$sys_dassgn");
int sys$exit(unsigned int code) __asm__("decc$$sys_exit");
int sys$get_entropy(void *buffer, unsigned int buflen) __asm__("decc$$sys_get_entropy");
int sys$gettim(void *timadr) __asm__("decc$$sys_gettim");
/* p1 to p6 by value, whatever they are: an address or a number. */
int sys$qiow(unsigned int efn, unsigned short chan, unsigned int func, void *iosb,
	     void (*astadr)(void *), void *astprm, ...) __asm__("decc$$sys_qiow");

#endif
