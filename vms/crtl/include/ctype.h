/* ctype.h: ASCII character classes. */
#ifndef __CTYPE_LOADED
#define __CTYPE_LOADED

#include <decc$types.h>

int isalnum(int c) __DECC(isalnum);
int isalpha(int c) __DECC(isalpha);
int isdigit(int c) __DECC(isdigit);
int islower(int c) __DECC(islower);
int isprint(int c) __DECC(isprint);
int isspace(int c) __DECC(isspace);
int isupper(int c) __DECC(isupper);
int isxdigit(int c) __DECC(isxdigit);
int tolower(int c) __DECC(tolower);
int toupper(int c) __DECC(toupper);

#endif
