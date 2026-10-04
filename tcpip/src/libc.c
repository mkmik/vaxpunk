/* The string functions include/string.h declares, and atoi; gcc may call these
 * for copy and fill loops too, even freestanding. */
#include <stdlib.h>
#include <string.h>

void *memcpy(void *dst, const void *src, size_t n)
{
	char *d = dst;
	const char *s = src;
	while (n--)
		*d++ = *s++;
	return dst;
}

void *memmove(void *dst, const void *src, size_t n)
{
	char *d = dst;
	const char *s = src;
	if (d < s)
		return memcpy(dst, src, n);
	while (n--)
		d[n] = s[n];
	return dst;
}

void *memset(void *dst, int c, size_t n)
{
	volatile char *d = dst;
	while (n--)
		*d++ = c;
	return dst;
}

int memcmp(const void *a, const void *b, size_t n)
{
	const unsigned char *x = a, *y = b;
	for (; n; n--, x++, y++)
		if (*x != *y)
			return *x - *y;
	return 0;
}

size_t strlen(const char *s)
{
	size_t n = 0;
	while (s[n])
		n++;
	return n;
}

int strncmp(const char *a, const char *b, size_t n)
{
	for (; n; n--, a++, b++)
		if (*a != *b || !*a)
			return (unsigned char)*a - (unsigned char)*b;
	return 0;
}

int strcmp(const char *a, const char *b)
{
	return strncmp(a, b, (size_t)-1);
}

int atoi(const char *s)
{
	int v = 0;
	while (*s >= '0' && *s <= '9')
		v = v * 10 + *s++ - '0';
	return v;
}
