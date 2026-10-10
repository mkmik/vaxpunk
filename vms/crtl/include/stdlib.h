/* stdlib.h: memory, conversions and the end of the program. */
#ifndef __STDLIB_LOADED
#define __STDLIB_LOADED

#include <decc$types.h>

/* exit(EXIT_SUCCESS) ends the image with SS$_NORMAL; EXIT_FAILURE is an
 * error whose message DCL doesn't print, as DEC C's. Any other value is
 * the condition value the image ends with. */
#define EXIT_SUCCESS 0
#define EXIT_FAILURE 0x10000002

__attribute__((noreturn)) void abort(void) __DECC(abort);
int abs(int j) __DECC(abs);
int atoi(const char *s) __DECC(atoi);
void *calloc(size_t n, size_t size) __DECC(calloc);
__attribute__((noreturn)) void exit(int status) __DECC(exit);
void free(void *p) __DECC(free);
/* A logical name's equivalence, as DEC C's getenv finds one: the
 * process's, then the system's. ponytail: no symbols, no HOME or PATH. */
char *getenv(const char *name) __DECC(getenv);
long labs(long j) __DECC(labs);
void *malloc(size_t size) __DECC(malloc);
void *realloc(void *p, size_t size) __DECC(realloc);
long strtol(const char *s, char **end, int base) __DECC(strtol);
unsigned long strtoul(const char *s, char **end, int base) __DECC(strtoul);

#endif
