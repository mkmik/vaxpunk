/*
 * The root task: the first and only user task seL4 starts, holding every
 * capability. For now it shows that seL4 handed it a valid boot info page,
 * then suspends itself.
 */
#include <stdarg.h>

#include <sel4/sel4.h>

/* Defined here instead of in libsel4, which the root task does not link. */
LIBSEL4_THREAD_LOCAL seL4_IPCBuffer *__sel4_ipc_buffer;

static void putnum(unsigned long v, unsigned base)
{
	char buf[20];
	int n = 0;
	do {
		buf[n++] = "0123456789abcdef"[v % base];
		v /= base;
	} while (v);
	while (n)
		seL4_DebugPutChar(buf[--n]);
}

/* printf subset: %s, %c, %u, %x, %lu, %lx. */
static void __attribute__((format(printf, 1, 2))) print(const char *fmt, ...)
{
	va_list ap;
	va_start(ap, fmt);
	for (; *fmt; fmt++) {
		if (*fmt != '%') {
			seL4_DebugPutChar(*fmt);
			continue;
		}
		int wide = *++fmt == 'l';
		fmt += wide;
		if (*fmt == 's')
			for (const char *s = va_arg(ap, const char *); *s; s++)
				seL4_DebugPutChar(*s);
		else if (*fmt == 'c')
			seL4_DebugPutChar(va_arg(ap, int));
		else
			putnum(wide ? va_arg(ap, unsigned long) : va_arg(ap, unsigned),
			       *fmt == 'x' ? 16 : 10);
	}
	va_end(ap);
}

int main(seL4_BootInfo *bi)
{
	seL4_SetIPCBuffer(bi->ipcBuffer);
	print("hello from the root task\n");

	seL4_Word n = bi->untyped.end - bi->untyped.start;
	print("boot info: node %lu of %lu, %lu untyped caps\n", bi->nodeID, bi->numNodes, n);
	for (seL4_Word i = 0; i < n; i++) {
		seL4_UntypedDesc *u = &bi->untypedList[i];
		print("  untyped %lu: paddr 0x%lx size 2^%u%s\n", i, u->paddr, u->sizeBits,
		      u->isDevice ? " device" : "");
	}

	print("root task done\n");
	seL4_Error err = seL4_TCB_Suspend(seL4_CapInitThreadTCB);
	print("seL4_TCB_Suspend failed: %u\n", err);
	return 0;
}
