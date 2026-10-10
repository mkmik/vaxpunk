/*
 * The C run-time library's memory and string routines, its character
 * classes (ASCII) and its conversions from text to numbers.
 */
#include <ctype.h>
#include <errno.h>
#include <limits.h>
#include <stdlib.h>
#include <string.h>

void *memchr(const void *s, int c, size_t n)
{
	const unsigned char *p = s;

	for (; n--; p++)
		if (*p == (unsigned char)c)
			return (void *)p;
	return NULL;
}

int memcmp(const void *a, const void *b, size_t n)
{
	const unsigned char *x = a, *y = b;

	for (; n--; x++, y++)
		if (*x != *y)
			return *x - *y;
	return 0;
}

void *memcpy(void *dst, const void *src, size_t n)
{
	unsigned char *d = dst;
	const unsigned char *s = src;

	while (n--)
		*d++ = *s++;
	return dst;
}

void *memmove(void *dst, const void *src, size_t n)
{
	unsigned char *d = dst;
	const unsigned char *s = src;

	if (d < s)
		while (n--)
			*d++ = *s++;
	else
		while (n--)
			d[n] = s[n];
	return dst;
}

void *memset(void *s, int c, size_t n)
{
	unsigned char *p = s;

	while (n--)
		*p++ = (unsigned char)c;
	return s;
}

/* gcc calls these four by their plain names for its own copies and
 * clears (velf.md), whatever string.h says. */
void *memcpy_(void *, const void *, size_t) __asm__("memcpy") __attribute__((alias("decc$memcpy")));
void *memmove_(void *, const void *, size_t) __asm__("memmove") __attribute__((alias("decc$memmove")));
void *memset_(void *, int, size_t) __asm__("memset") __attribute__((alias("decc$memset")));
int memcmp_(const void *, const void *, size_t) __asm__("memcmp") __attribute__((alias("decc$memcmp")));

size_t strlen(const char *s)
{
	size_t n = 0;

	while (s[n])
		n++;
	return n;
}

int strcmp(const char *a, const char *b)
{
	for (; *a && *a == *b; a++, b++)
		;
	return (unsigned char)*a - (unsigned char)*b;
}

int strncmp(const char *a, const char *b, size_t n)
{
	for (; n; n--, a++, b++)
		if (*a != *b || !*a)
			return (unsigned char)*a - (unsigned char)*b;
	return 0;
}

char *strchr(const char *s, int c)
{
	for (;; s++) {
		if (*s == (char)c)
			return (char *)s;
		if (!*s)
			return NULL;
	}
}

char *strrchr(const char *s, int c)
{
	const char *last = NULL;

	for (;; s++) {
		if (*s == (char)c)
			last = s;
		if (!*s)
			return (char *)last;
	}
}

char *strstr(const char *haystack, const char *needle)
{
	size_t n = strlen(needle);

	for (; *haystack; haystack++)
		if (!strncmp(haystack, needle, n))
			return (char *)haystack;
	return n ? NULL : (char *)haystack;
}

char *strcpy(char *dst, const char *src)
{
	return memcpy(dst, src, strlen(src) + 1);
}

char *strncpy(char *dst, const char *src, size_t n)
{
	size_t k = 0;

	for (; k < n && src[k]; k++)
		dst[k] = src[k];
	for (; k < n; k++)
		dst[k] = 0;
	return dst;
}

char *strcat(char *dst, const char *src)
{
	strcpy(dst + strlen(dst), src);
	return dst;
}

char *strncat(char *dst, const char *src, size_t n)
{
	char *d = dst + strlen(dst);

	while (n-- && *src)
		*d++ = *src++;
	*d = 0;
	return dst;
}

int isdigit(int c) { return c >= '0' && c <= '9'; }
int islower(int c) { return c >= 'a' && c <= 'z'; }
int isupper(int c) { return c >= 'A' && c <= 'Z'; }
int isalpha(int c) { return islower(c) || isupper(c); }
int isalnum(int c) { return isalpha(c) || isdigit(c); }
int isprint(int c) { return c >= ' ' && c < 127; }
int isspace(int c) { return c == ' ' || (c >= '\t' && c <= '\r'); }
int isxdigit(int c) { return isdigit(c) || ((c | 32) >= 'a' && (c | 32) <= 'f'); }
int tolower(int c) { return isupper(c) ? c + 32 : c; }
int toupper(int c) { return islower(c) ? c - 32 : c; }

int abs(int j) { return j < 0 ? -j : j; }
long labs(long j) { return j < 0 ? -j : j; }

/*
 * The number at s in base, 2 to 36, or 0 for C's prefixes, after blanks
 * and a sign, as an unsigned magnitude; *neg if it had a minus. Sets *end
 * past it, or to s if there is none, and errno to ERANGE past limit.
 */
static unsigned long number(const char *s, char **end, int base, unsigned long limit, int *neg)
{
	const char *p = s;
	unsigned long v = 0;
	int any = 0, over = 0;

	while (isspace(*p))
		p++;
	*neg = *p == '-';
	if (*p == '-' || *p == '+')
		p++;
	if ((base == 0 || base == 16) && p[0] == '0' && (p[1] | 32) == 'x' && isxdigit(p[2]))
		p += 2, base = 16;
	else if (base == 0)
		base = *p == '0' ? 8 : 10;
	for (;; p++, any = 1) {
		int d = isdigit(*p) ? *p - '0' : isalpha(*p) ? (*p | 32) - 'a' + 10 : 99;
		if (d >= base)
			break;
		if (v > (limit - d) / base)
			over = 1;
		else
			v = v * base + d;
	}
	if (end)
		*end = (char *)(any ? p : s);
	if (over) {
		errno = ERANGE;
		return limit;
	}
	return v;
}

long strtol(const char *s, char **end, int base)
{
	int neg;
	unsigned long v = number(s, end, base, (unsigned long)LONG_MAX + 1, &neg);

	if (!neg && v > LONG_MAX) {
		errno = ERANGE;
		return LONG_MAX;
	}
	return neg ? (long)(0 - v) : (long)v;
}

unsigned long strtoul(const char *s, char **end, int base)
{
	int neg;
	unsigned long v = number(s, end, base, ULONG_MAX, &neg);

	return neg ? -v : v;
}

int atoi(const char *s)
{
	return (int)strtol(s, NULL, 10);
}
