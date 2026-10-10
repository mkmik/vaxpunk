/* openssl/crypto.h: the library's start, its version and its memory. */
#ifndef OPENSSL_CRYPTO_H
#define OPENSSL_CRYPTO_H

#include <stdint.h>
#include <stdlib.h>
#include <openssl/opensslv.h>

#define OPENSSL_VERSION 0

#define OPENSSL_malloc(n) malloc(n)
#define OPENSSL_free(p) free(p)

const char *OpenSSL_version(int type);
unsigned long OpenSSL_version_num(void);

#endif
