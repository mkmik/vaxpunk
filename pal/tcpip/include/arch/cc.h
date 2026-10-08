/* lwIP's view of the platform: freestanding AArch64, no libc but
 * src/libc.c's string functions. */
#ifndef ARCH_CC_H
#define ARCH_CC_H

#define LWIP_NO_INTTYPES_H 1
#define LWIP_NO_CTYPE_H 1
#define BYTE_ORDER LITTLE_ENDIAN

void tcpip_print(const char *fmt, ...);
void tcpip_halt(void) __attribute__((noreturn));
unsigned tcpip_rand(void);

#define LWIP_PLATFORM_DIAG(x) \
	do { \
		tcpip_print x; \
	} while (0)
#define LWIP_PLATFORM_ASSERT(x) \
	do { \
		tcpip_print("tcpip: assertion \"%s\" failed at %s:%u\n", x, __FILE__, __LINE__); \
		tcpip_halt(); \
	} while (0)
#define LWIP_RAND() ((u32_t)tcpip_rand())

#endif
