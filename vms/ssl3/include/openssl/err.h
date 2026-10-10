/* openssl/err.h: the error queue. Each error is OpenSSL's packing of a
 * library and a reason: ERR_LIB_SSL's reasons, or an Mbed TLS error code
 * with ERR_R_MBEDTLS. */
#ifndef OPENSSL_ERR_H
#define OPENSSL_ERR_H

#include <stddef.h>
#include <stdio.h>

#define ERR_LIB_SSL 20
#define ERR_PACK(lib, func, reason) ((unsigned long)((lib) & 0xff) << 23 | ((reason) & 0x7fffff))
#define ERR_GET_LIB(e) (int)(((e) >> 23) & 0xff)
#define ERR_GET_REASON(e) (int)((e) & 0x7fffff)

/* vaxpunk: a reason with this bit is an Mbed TLS error code, negated. */
#define ERR_R_MBEDTLS 0x100000

#define SSL_R_CERTIFICATE_VERIFY_FAILED 134
#define SSL_R_UNEXPECTED_EOF_WHILE_READING 294

unsigned long ERR_get_error(void);
unsigned long ERR_peek_error(void);
void ERR_clear_error(void);
char *ERR_error_string(unsigned long e, char *buf);
void ERR_error_string_n(unsigned long e, char *buf, size_t len);
const char *ERR_reason_error_string(unsigned long e);
void ERR_print_errors_fp(FILE *fp);

#define ERR_load_crypto_strings() ((void)0)
#define ERR_free_strings() ((void)0)

#endif
