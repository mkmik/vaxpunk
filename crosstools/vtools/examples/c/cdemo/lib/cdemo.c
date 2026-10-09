/*
 * CDEMO: a C library as vaxpunk would port one. A stock gcc compiles it
 * with C's calling conventions and velf turns the result into an object
 * module; cdemo_vms.b64 puts VMS's conventions on it. The bodies are
 * trivial: the signatures are the point. They take arguments of every
 * width and sign, some on the stack, return results narrower than a
 * fullword, write through pointers, keep a table of addresses and a
 * global, and call back into their caller.
 */

/* How many times the library was called. BLISS reads it as a longword. */
int cdemo_calls;

/* Addresses in read-only data: velf puts the table in $READONLY_ADDR$,
 * which isn't PIC, so an image that moves gets a fixup for each. */
static const char *const names[] = {
	"zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
};

/*
 * The sum of ten integers of every width and sign; the smallest of them
 * through minimum. The ninth and tenth arguments and minimum come on the
 * stack.
 */
long cdemo_sum(int a, long b, signed char c, unsigned char d, short e, unsigned short f, int g,
	       int h, int i, long j, int *minimum)
{
	long v[] = { a, b, c, d, e, f, g, h, i, j };
	long sum = 0, min = v[0];

	cdemo_calls++;
	for (int k = 0; k < 10; k++) {
		sum += v[k];
		if (v[k] < min)
			min = v[k];
	}
	*minimum = (int)min;
	return sum;
}

/*
 * Copies the English name of n, 0 to 9, to the size bytes at buf. Returns
 * its length, or -1 for any other n or a buffer too small.
 */
int cdemo_name(int n, char *buf, unsigned long size)
{
	cdemo_calls++;
	if (n < 0 || n > 9)
		return -1;
	const char *s = names[n];
	unsigned long len = 0;
	while (s[len])
		len++;
	if (len > size)
		return -1;
	for (unsigned long k = 0; k < len; k++)
		buf[k] = s[k];
	return (int)len;
}

/*
 * Calls emit for each word of the len bytes at text, a word being a run
 * of anything but spaces, until emit returns nonzero. Returns how many
 * words emit took, or what emit returned to stop, with the offset of the
 * word it stopped at in *stop.
 */
int cdemo_words(const char *text, unsigned long len,
		int (*emit)(void *ctx, const char *word, int len), void *ctx,
		unsigned long *stop)
{
	int n = 0;

	cdemo_calls++;
	for (unsigned long k = 0; k < len;) {
		while (k < len && text[k] == ' ')
			k++;
		unsigned long start = k;
		while (k < len && text[k] != ' ')
			k++;
		if (k > start) {
			int r = emit(ctx, text + start, (int)(k - start));
			if (r) {
				*stop = start;
				return r;
			}
			n++;
		}
	}
	return n;
}
