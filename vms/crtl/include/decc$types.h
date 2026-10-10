/* decc$types.h: what the C run-time library's headers share. */
#ifndef __DECC_TYPES_LOADED
#define __DECC_TYPES_LOADED

#include <stddef.h>
#include <stdint.h>

/* A library routine's name is decc$name, as DEC C's
 * /PREFIX_LIBRARY_ENTRIES=ALL_ENTRIES makes calls to it, so that a
 * program's own names never meet the library's. */
#define __DECC(name) __asm__("decc$" #name)

#endif
