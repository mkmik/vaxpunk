/* stdio.h: streams. stdout and stderr are SYS$OUTPUT and SYS$ERROR, a
 * record each line; stdin is SYS$INPUT; fopen opens a file through RMS. */
#ifndef __STDIO_LOADED
#define __STDIO_LOADED

#include <decc$types.h>
#include <stdarg.h>

typedef struct _iobuf FILE;

#define EOF (-1)
#define BUFSIZ 512
#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2

extern FILE *stdin __DECC(ga_stdin);
extern FILE *stdout __DECC(ga_stdout);
extern FILE *stderr __DECC(ga_stderr);

int fclose(FILE *f) __DECC(fclose);
int feof(FILE *f) __DECC(feof);
int ferror(FILE *f) __DECC(ferror);
int fflush(FILE *f) __DECC(fflush);
int fgetc(FILE *f) __DECC(fgetc);
char *fgets(char *s, int n, FILE *f) __DECC(fgets);
FILE *fopen(const char *name, const char *mode) __DECC(fopen);
__attribute__((format(printf, 2, 3))) int fprintf(FILE *f, const char *fmt, ...) __DECC(fprintf);
int fputc(int c, FILE *f) __DECC(fputc);
int fputs(const char *s, FILE *f) __DECC(fputs);
size_t fread(void *p, size_t size, size_t n, FILE *f) __DECC(fread);
size_t fwrite(const void *p, size_t size, size_t n, FILE *f) __DECC(fwrite);
int getc(FILE *f) __DECC(getc);
int getchar(void) __DECC(getchar);
void perror(const char *s) __DECC(perror);
__attribute__((format(printf, 1, 2))) int printf(const char *fmt, ...) __DECC(printf);
int putc(int c, FILE *f) __DECC(putc);
int putchar(int c) __DECC(putchar);
int puts(const char *s) __DECC(puts);
__attribute__((format(printf, 3, 4))) int snprintf(char *s, size_t n, const char *fmt, ...) __DECC(snprintf);
__attribute__((format(printf, 2, 3))) int sprintf(char *s, const char *fmt, ...) __DECC(sprintf);
int vfprintf(FILE *f, const char *fmt, va_list ap) __DECC(vfprintf);
int vprintf(const char *fmt, va_list ap) __DECC(vprintf);
int vsnprintf(char *s, size_t n, const char *fmt, va_list ap) __DECC(vsnprintf);
int vsprintf(char *s, const char *fmt, va_list ap) __DECC(vsprintf);

#endif
