/*
 * A C program's start, a module of its own, so that a MACRO-32 or BLISS
 * program that calls the library has no main to find.
 */
#include <ctype.h>
#include <stdlib.h>
#include "crtl.h"

/*
 * DECC$$START: the program, from decc$main.mar's transfer address. Its
 * arguments are the image's name and the words of the foreign command
 * line that ran it, in lower case unless in quotes, as DEC C's are.
 */
int main(int argc, char **argv);

void decc$$start(void)
{
	static char line[256], *argv[33];
	unsigned len = 0;
	int argc = 0;

	argv[argc++] = "image";	/* ponytail: not the image's file name */
	decc$$foreign(line, sizeof line - 1, &len);
	for (char *p = line, *end = line + len; p < end && argc < 32;) {
		if (*p == ' ' || *p == '\t') {
			p++;
			continue;
		}
		char *w = p, *d = p;
		int quoted = 0;
		for (; p < end && (quoted || (*p != ' ' && *p != '\t')); p++) {
			if (*p == '"')
				quoted = !quoted;
			else
				*d++ = quoted ? *p : (char)tolower(*p);
		}
		if (p < end)
			p++;
		*d = 0;
		argv[argc++] = w;
	}
	argv[argc] = NULL;
	exit(main(argc, argv));
}
