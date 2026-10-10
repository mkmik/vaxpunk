/* What SSL3's two modules share. */
#ifndef SSL3_H
#define SSL3_H

#include <stdint.h>
#include <mbedtls/x509_crt.h>

/* An X509 is a certificate of its own, a copy of the peer's. */
struct x509_st {
	mbedtls_x509_crt crt;
};

/* Puts an error on the queue: an OpenSSL code, or an Mbed TLS one. */
void ssl3$push_error(unsigned long e);
void ssl3$push_mbedtls(int ret);
/* Mbed TLS's verification flags as an X509_V_ result. */
long ssl3$verify_result(uint32_t flags);

#endif
