/* string.h: memory and strings. */
#ifndef __STRING_LOADED
#define __STRING_LOADED

#include <decc$types.h>

void *memchr(const void *s, int c, size_t n) __DECC(memchr);
int memcmp(const void *a, const void *b, size_t n) __DECC(memcmp);
void *memcpy(void *dst, const void *src, size_t n) __DECC(memcpy);
void *memmove(void *dst, const void *src, size_t n) __DECC(memmove);
void *memset(void *s, int c, size_t n) __DECC(memset);
char *strcat(char *dst, const char *src) __DECC(strcat);
char *strchr(const char *s, int c) __DECC(strchr);
int strcmp(const char *a, const char *b) __DECC(strcmp);
char *strcpy(char *dst, const char *src) __DECC(strcpy);
char *strerror(int errnum) __DECC(strerror);
size_t strlen(const char *s) __DECC(strlen);
char *strncat(char *dst, const char *src, size_t n) __DECC(strncat);
int strncmp(const char *a, const char *b, size_t n) __DECC(strncmp);
char *strncpy(char *dst, const char *src, size_t n) __DECC(strncpy);
char *strrchr(const char *s, int c) __DECC(strrchr);
char *strstr(const char *haystack, const char *needle) __DECC(strstr);

#endif
