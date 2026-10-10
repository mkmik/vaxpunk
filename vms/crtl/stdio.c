/*
 * The C run-time library's streams, as DEC C's are on a record-oriented
 * system: stdout and stderr gather a line and write it to SYS$OUTPUT as a
 * record; stdin reads a record from SYS$INPUT, with what stdout gathered
 * as its prompt, and gives it as a line; a file fopen opens is read a
 * record at a time through RMS, each a line. printf and its family
 * format into any of them.
 *
 * ponytail: stderr goes to SYS$OUTPUT, which is where SYS$ERROR goes
 * interactively; fopen opens files to read only; no floating point
 * conversions, which code built without FP registers can't have.
 */
#include <errno.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include "crtl.h"

#define RMS$_EOF 0x1827a
#define RMS$_RTB 0x1866b

static FILE out = { .kind = OUTPUT }, err = { .kind = OUTPUT }, in = { .kind = INPUT };
FILE *stdin = &in, *stdout = &out, *stderr = &err;

/* Writes what f gathered as a record. */
static int flush(FILE *f)
{
	int status;

	if (f->kind != OUTPUT || f->len == 0)
		return 0;
	status = decc$$put(f->buf, f->len);
	f->len = 0;
	if (!(status & 1)) {
		f->err = 1;
		errno = EIO;
		return EOF;
	}
	return 0;
}

int fflush(FILE *f)
{
	if (f)
		return flush(f);
	decc$$flush_all();
	return 0;
}

void decc$$flush_all(void)
{
	flush(stdout);
	flush(stderr);
}

int fputc(int c, FILE *f)
{
	if (f->kind != OUTPUT) {
		errno = EBADF;
		return EOF;
	}
	if (c == '\n')
		return flush(f) ? EOF : c;
	if (f->len == STREAM_MAX && flush(f))
		return EOF;
	f->buf[f->len++] = (char)c;
	return (unsigned char)c;
}

int putc(int c, FILE *f) { return fputc(c, f); }
int putchar(int c) { return fputc(c, stdout); }

size_t fwrite(const void *p, size_t size, size_t n, FILE *f)
{
	const unsigned char *s = p;

	for (size_t k = 0; k < size * n; k++)
		if (fputc(s[k], f) == EOF)
			return size ? k / size : 0;
	return n;
}

int fputs(const char *s, FILE *f)
{
	size_t n = strlen(s);

	return fwrite(s, 1, n, f) == n ? 0 : EOF;
}

int puts(const char *s)
{
	return fputs(s, stdout) == EOF || fputc('\n', stdout) == EOF ? EOF : 0;
}

/* Reads f's next record into buf, a line. 0 at the end of the file. */
static int fill(FILE *f)
{
	unsigned len = 0;
	int status;

	if (f->eof || f->err)
		return 0;
	if (f->kind == INPUT) {
		/* What stdout gathered is the prompt, as DEC C writes it. */
		status = decc$$get(f->buf, STREAM_MAX, stdout->buf, stdout->len, &len);
		stdout->len = 0;
		if (status == SS$_ENDOFFILE)
			status = RMS$_EOF;
	} else {
		status = decc$$read(f->rms, f->buf, STREAM_MAX, &len);
		if (status == RMS$_RTB)	/* ponytail: a record past STREAM_MAX is cut */
			status = SS$_NORMAL, len = STREAM_MAX;
	}
	if (status == RMS$_EOF) {
		f->eof = 1;
		return 0;
	}
	if (!(status & 1)) {
		f->err = 1;
		errno = EIO;
		return 0;
	}
	f->buf[len] = '\n';
	f->len = len + 1;
	f->at = 0;
	return 1;
}

int fgetc(FILE *f)
{
	if (f->kind != INPUT && f->kind != FILE_READ) {
		errno = EBADF;
		return EOF;
	}
	if (f->at == f->len && !fill(f))
		return EOF;
	return (unsigned char)f->buf[f->at++];
}

int getc(FILE *f) { return fgetc(f); }
int getchar(void) { return fgetc(stdin); }

char *fgets(char *s, int n, FILE *f)
{
	int k = 0, c = 0;

	while (k < n - 1 && c != '\n' && (c = fgetc(f)) != EOF)
		s[k++] = (char)c;
	if (k == 0)
		return NULL;
	s[k] = 0;
	return s;
}

size_t fread(void *p, size_t size, size_t n, FILE *f)
{
	unsigned char *d = p;
	size_t k;
	int c;

	for (k = 0; k < size * n && (c = fgetc(f)) != EOF; k++)
		d[k] = (unsigned char)c;
	return size ? k / size : 0;
}

int feof(FILE *f) { return f->eof; }
int ferror(FILE *f) { return f->err; }

FILE *fopen(const char *name, const char *mode)
{
	FILE *f;
	int status;

	if (mode[0] != 'r' || strchr(mode, '+')) {
		errno = EINVAL;
		return NULL;
	}
	f = calloc(1, sizeof *f);
	if (!f)
		return NULL;
	f->kind = FILE_READ;
	status = decc$$open(f->rms, name, strlen(name));
	if (!(status & 1)) {
		free(f);
		errno = ENOENT;
		return NULL;
	}
	return f;
}

int fclose(FILE *f)
{
	int r = 0;

	if (f->kind == FILE_READ) {
		decc$$close(f->rms);
		free(f);
	} else {
		r = flush(f);
	}
	return r;
}

/* printf's engine: writes through put, one byte at a time, to ctx. */
typedef void put_fn(void *ctx, char c);

static void pad(put_fn *put, void *ctx, char c, int n)
{
	while (n-- > 0)
		put(ctx, c);
}

static int format(put_fn *put, void *ctx, const char *fmt, va_list ap)
{
	int total = 0;

#define PUT(c) (put(ctx, (c)), total++)
	for (; *fmt; fmt++) {
		if (*fmt != '%') {
			PUT(*fmt);
			continue;
		}
		int left = 0, zero = 0, plus = 0, space = 0, alt = 0, width = 0, prec = -1, size = 0;
		for (;; fmt++) {
			if (fmt[1] == '-') left = 1;
			else if (fmt[1] == '0') zero = 1;
			else if (fmt[1] == '+') plus = 1;
			else if (fmt[1] == ' ') space = 1;
			else if (fmt[1] == '#') alt = 1;
			else break;
		}
		fmt++;
		if (*fmt == '*') {
			width = va_arg(ap, int);
			if (width < 0)
				left = 1, width = -width;
			fmt++;
		} else {
			while (*fmt >= '0' && *fmt <= '9')
				width = width * 10 + *fmt++ - '0';
		}
		if (*fmt == '.') {
			fmt++;
			prec = 0;
			if (*fmt == '*') {
				prec = va_arg(ap, int);
				fmt++;
			} else {
				while (*fmt >= '0' && *fmt <= '9')
					prec = prec * 10 + *fmt++ - '0';
			}
		}
		/* size: -2 hh, -1 h, 0 int, 1 long and the 64-bit ones */
		for (;; fmt++) {
			if (*fmt == 'h') size--;
			else if (*fmt == 'l' || *fmt == 'z' || *fmt == 'j' || *fmt == 't' || *fmt == 'L') size = 1;
			else break;
		}
		char digits[24], *s = digits;
		int len = 0, base = 10, neg = 0, upper = 0;
		uint64_t v = 0;
		const char *prefix = "";
		switch (*fmt) {
		case 'd': case 'i': {
			int64_t n = size > 0 ? va_arg(ap, long) : va_arg(ap, int);
			if (size == -1) n = (short)n;
			if (size <= -2) n = (signed char)n;
			neg = n < 0;
			v = neg ? 0 - (uint64_t)n : (uint64_t)n;
			prefix = neg ? "-" : plus ? "+" : space ? " " : "";
			goto number;
		}
		case 'p':
			v = (uintptr_t)va_arg(ap, void *);
			base = 16, prefix = "0x";
			goto digits;
		case 'X':
			upper = 1;
			/* fall through */
		case 'x':
			base = 16;
			goto unsigned_;
		case 'o':
			base = 8;
			goto unsigned_;
		case 'u':
		unsigned_:
			v = size > 0 ? va_arg(ap, unsigned long) : va_arg(ap, unsigned int);
			if (size == -1) v = (unsigned short)v;
			if (size <= -2) v = (unsigned char)v;
			if (alt && v && base == 16) prefix = upper ? "0X" : "0x";
			if (alt && base == 8) prefix = "0";
		number:
		digits:
			s = digits + sizeof digits;
			do {
				int d = v % base;
				*--s = (char)(d < 10 ? '0' + d : (upper ? 'A' : 'a') + d - 10);
				v /= base;
			} while (v);
			len = digits + sizeof digits - s;
			if (prec == 0 && len == 1 && *s == '0')
				len = 0;
			{
				int zeros = prec > len ? prec - len : 0;
				int plen = strlen(prefix);
				int fill = width - plen - zeros - len;
				if (zero && !left && prec < 0)
					zeros += fill > 0 ? fill : 0, fill = 0;
				if (!left)
					pad(put, ctx, ' ', fill), total += fill > 0 ? fill : 0;
				for (; *prefix; prefix++)
					PUT(*prefix);
				pad(put, ctx, '0', zeros), total += zeros;
				for (int k = 0; k < len; k++)
					PUT(s[k]);
				if (left)
					pad(put, ctx, ' ', fill), total += fill > 0 ? fill : 0;
			}
			break;
		case 'c':
			digits[0] = (char)va_arg(ap, int);
			len = 1;
			goto string;
		case 's':
			s = va_arg(ap, char *);
			if (!s)
				s = "(null)";
			while ((prec < 0 || len < prec) && s[len])
				len++;
		string:
			if (!left)
				pad(put, ctx, ' ', width - len), total += width > len ? width - len : 0;
			for (int k = 0; k < len; k++)
				PUT(s[k]);
			if (left)
				pad(put, ctx, ' ', width - len), total += width > len ? width - len : 0;
			break;
		case '%':
			PUT('%');
			break;
		default:	/* ponytail: no %e, %f, %g or %n */
			if (!*fmt)
				return total;
			PUT('%');
			PUT(*fmt);
		}
	}
	return total;
#undef PUT
}

struct buffer {
	char *s;
	size_t n, at;
};

static void put_buffer(void *ctx, char c)
{
	struct buffer *b = ctx;

	if (b->at + 1 < b->n)
		b->s[b->at] = c;
	b->at++;
}

static void put_stream(void *ctx, char c)
{
	fputc(c, ctx);
}

int vsnprintf(char *s, size_t n, const char *fmt, va_list ap)
{
	struct buffer b = { s, n, 0 };
	int total = format(put_buffer, &b, fmt, ap);

	if (n)
		s[b.at < n ? b.at : n - 1] = 0;
	return total;
}

int vsprintf(char *s, const char *fmt, va_list ap)
{
	return vsnprintf(s, SIZE_MAX, fmt, ap);
}

int vfprintf(FILE *f, const char *fmt, va_list ap)
{
	return format(put_stream, f, fmt, ap);
}

int vprintf(const char *fmt, va_list ap)
{
	return vfprintf(stdout, fmt, ap);
}

int snprintf(char *s, size_t n, const char *fmt, ...)
{
	va_list ap;
	va_start(ap, fmt);
	int r = vsnprintf(s, n, fmt, ap);
	va_end(ap);
	return r;
}

int sprintf(char *s, const char *fmt, ...)
{
	va_list ap;
	va_start(ap, fmt);
	int r = vsprintf(s, fmt, ap);
	va_end(ap);
	return r;
}

int fprintf(FILE *f, const char *fmt, ...)
{
	va_list ap;
	va_start(ap, fmt);
	int r = vfprintf(f, fmt, ap);
	va_end(ap);
	return r;
}

int printf(const char *fmt, ...)
{
	va_list ap;
	va_start(ap, fmt);
	int r = vfprintf(stdout, fmt, ap);
	va_end(ap);
	return r;
}

void perror(const char *s)
{
	if (s && *s)
		fprintf(stderr, "%s: ", s);
	fprintf(stderr, "%s\n", strerror(errno));
}
