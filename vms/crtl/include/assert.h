/* assert.h: assert(e) prints where e failed and aborts, unless NDEBUG. */
#undef assert
#ifdef NDEBUG
#define assert(e) ((void)0)
#else
#include <decc$types.h>
__attribute__((noreturn)) void __assert(const char *e, const char *file, int line) __DECC(__assert);
#define assert(e) ((e) ? (void)0 : __assert(#e, __FILE__, __LINE__))
#endif
