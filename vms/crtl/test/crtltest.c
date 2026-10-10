/*
 * CRTLTEST: checks the C run-time library's routines that SSL3$CLIENT
 * leaves alone or uses only one way: printf's conversions, strtol, the
 * dates gmtime_r gives, malloc's free list as it grows, and a file read
 * through RMS. Prints CRTLTEST: ok, or what it got and what it wanted.
 * The boot test runs it.
 */
#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static int failed;

static void check(const char *what, const char *got, const char *want)
{
	if (strcmp(got, want) != 0 && !failed++)
		printf("CRTLTEST: %s is \"%s\", not \"%s\"\n", what, got, want);
}

static void check_long(const char *what, long got, long want)
{
	char g[24], w[24];

	snprintf(g, sizeof g, "%ld", got);
	snprintf(w, sizeof w, "%ld", want);
	check(what, g, w);
}

static void dates(void)
{
	static const struct {
		time_t t;
		const char *want;
	} cases[] = {
		{ 0, "70 0 1 0:0:0 4 0" },
		{ 951782400, "100 1 29 0:0:0 2 59" },	/* 29-FEB-2000 */
		{ 4102444799, "199 11 31 23:59:59 4 364" },
		{ -1, "69 11 31 23:59:59 3 364" },
	};
	struct tm tm;
	char got[64];

	for (size_t k = 0; k < sizeof cases / sizeof cases[0]; k++) {
		gmtime_r(&cases[k].t, &tm);
		snprintf(got, sizeof got, "%d %d %d %d:%d:%d %d %d", tm.tm_year, tm.tm_mon,
			 tm.tm_mday, tm.tm_hour, tm.tm_min, tm.tm_sec, tm.tm_wday, tm.tm_yday);
		check("gmtime_r", got, cases[k].want);
	}
	if (time(NULL) < 1577836800 && !failed++)
		printf("CRTLTEST: time() is before 2020\n");
}

static void formats(void)
{
	char s[128];
	int n;

	snprintf(s, sizeof s, "%d|%5d|%-5d|%05d|%+d|% d|%x|%X|%#x|%o|%#o|%u", -42, 42, 42, -42,
		 7, 7, 255, 255, 255, 8, 8, 4000000000u);
	check("integers", s, "-42|   42|42   |-0042|+7| 7|ff|FF|0xff|10|010|4000000000");
	snprintf(s, sizeof s, "%ld|%lu|%hd|%hhu|%zu|%.3d|%.0d|%*d|%-*d|", LONG_MIN, ULONG_MAX,
		 (short)70000, (unsigned char)300, sizeof(long), 7, 0, 4, 1, 3, 2);
	check("sizes", s,
	      "-9223372036854775808|18446744073709551615|4464|44|8|007||   1|2  |");
	snprintf(s, sizeof s, "%c%c|%s|%.3s|%6s|%-6s|%%|%p|%s", 'o', 'k', "str", "string", "ab",
		 "ab", (void *)0x1f, (char *)NULL);
	check("text", s, "ok|str|str|    ab|ab    |%|0x1f|(null)");
	n = snprintf(s, 5, "hello, world");
	check("cut", s, "hell");
	check_long("cut's count", n, 12);
}

static void numbers(void)
{
	char *end;

	check_long("strtol", strtol("  -123abc", &end, 10), -123);
	check("strtol's end", end, "abc");
	check_long("strtol 0x", strtol("0x1f", NULL, 0), 31);
	check_long("strtol 0", strtol("077", NULL, 0), 63);
	check_long("strtol 36", strtol("zz", NULL, 36), 1295);
	errno = 0;
	check_long("strtol over", strtol("99999999999999999999", NULL, 10), LONG_MAX);
	check_long("its errno", errno, ERANGE);
	check_long("strtol min", strtol("-9223372036854775808", NULL, 10), LONG_MIN);
	check_long("strtoul -1", (long)strtoul("-1", NULL, 10), -1);
	check_long("atoi", atoi("2026"), 2026);
	strtol("none", &end, 10);
	check("no number's end", end, "none");
}

static void strings(void)
{
	char s[16] = "abcdefgh";

	memmove(s + 2, s, 5);
	check("memmove up", s, "ababcdeh");
	memmove(s, s + 2, 5);
	check("memmove down", s, "abcdedeh");
	check("strstr", strstr("vax punk vaxpunk", "vaxp"), "vaxpunk");
	check("strrchr", strrchr("a.b.c", '.'), ".c");
	strncpy(s, "xy", 4);
	check_long("strncpy's padding", s[2] | s[3], 0);
	strcpy(s, "con");
	check("strcat", strncat(strcat(s, "cat"), "enated", 3), "concatena");
}

/* Blocks of many sizes, half freed and the rest grown, past what one
 * $EXPREG gives: each must keep its bytes. */
static void memory(void)
{
	enum { N = 200 };
	unsigned char *p[N];
	size_t size[N];

	for (int k = 0; k < N; k++) {
		size[k] = (size_t)(k * 37 % 1500) + 1;
		p[k] = malloc(size[k]);
		if (!p[k]) {
			check("malloc", "NULL", "memory");
			return;
		}
		memset(p[k], k, size[k]);
	}
	for (int k = 0; k < N; k += 2)
		free(p[k]);
	for (int k = 1; k < N; k += 2) {
		p[k] = realloc(p[k], size[k] + 3000);
		memset(p[k] + size[k], k, 3000);
		size[k] += 3000;
	}
	for (int k = 1; k < N; k += 2)
		for (size_t i = 0; i < size[k]; i++)
			if (p[k][i] != k) {
				check("a block's bytes", "changed", "kept");
				return;
			}
	for (int k = 1; k < N; k += 2)
		free(p[k]);
	void *big = calloc(100, 1024);
	check("calloc", big && !((char *)big)[102399] ? "zeroed" : "not", "zeroed");
	free(big);
}

static void files(void)
{
	char line[100];
	FILE *f = fopen("SYS$MANAGER:WELCOME.TXT", "r");

	if (!f) {
		check("fopen", "NULL", "SYS$MANAGER:WELCOME.TXT");
		return;
	}
	fgets(line, sizeof line, f);
	check("its first line", line, "\n");
	fgets(line, sizeof line, f);
	check("its second line", strstr(line, "Welcome to vaxpunk") ? "welcome" : line, "welcome");
	fclose(f);
	errno = 0;
	f = fopen("SYS$MANAGER:NOSUCH.TXT", "r");
	check("fopen of none", f ? "a file" : strerror(errno), "no such file or directory");
}

int main(void)
{
	formats();
	numbers();
	strings();
	dates();
	memory();
	files();
	if (!failed)
		printf("CRTLTEST: ok\n");
	return failed ? EXIT_FAILURE : EXIT_SUCCESS;
}
