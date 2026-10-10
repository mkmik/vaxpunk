/* openssl/ssl.h: TLS connections over sockets, as OpenSSL 3.0's libssl.
 * vaxpunk's SSL3 is a TLS 1.2 and 1.3 client. */
#ifndef OPENSSL_SSL_H
#define OPENSSL_SSL_H

#include <stdint.h>
#include <openssl/crypto.h>
#include <openssl/x509.h>

typedef struct ssl_st SSL;
typedef struct ssl_ctx_st SSL_CTX;
typedef struct ssl_method_st SSL_METHOD;
typedef struct ssl_cipher_st SSL_CIPHER;

#define TLS1_2_VERSION 0x0303
#define TLS1_3_VERSION 0x0304

#define SSL_VERIFY_NONE 0
#define SSL_VERIFY_PEER 1
#define SSL_VERIFY_FAIL_IF_NO_PEER_CERT 2

/* SSL_CTX_set_options's: a peer that closes without close_notify ends
 * the data, SSL_ERROR_ZERO_RETURN, rather than an error. */
#define SSL_OP_IGNORE_UNEXPECTED_EOF ((uint64_t)1 << 7)

#define SSL_ERROR_NONE 0
#define SSL_ERROR_SSL 1
#define SSL_ERROR_WANT_READ 2
#define SSL_ERROR_WANT_WRITE 3
#define SSL_ERROR_SYSCALL 5
#define SSL_ERROR_ZERO_RETURN 6

#define OPENSSL_INIT_LOAD_SSL_STRINGS 0x00200000L
#define OPENSSL_INIT_LOAD_CRYPTO_STRINGS 0x00000002L

int OPENSSL_init_ssl(uint64_t opts, const void *settings);
#define SSL_library_init() OPENSSL_init_ssl(0, NULL)
#define SSL_load_error_strings() \
	OPENSSL_init_ssl(OPENSSL_INIT_LOAD_SSL_STRINGS | OPENSSL_INIT_LOAD_CRYPTO_STRINGS, NULL)
#define OpenSSL_add_ssl_algorithms() SSL_library_init()
#define SSLeay_add_ssl_algorithms() SSL_library_init()

const SSL_METHOD *TLS_method(void);
const SSL_METHOD *TLS_client_method(void);
#define SSLv23_method TLS_method
#define SSLv23_client_method TLS_client_method

SSL_CTX *SSL_CTX_new(const SSL_METHOD *method);
void SSL_CTX_free(SSL_CTX *ctx);
/* mode is SSL_VERIFY_NONE or SSL_VERIFY_PEER; vaxpunk: callback must be
 * NULL. */
void SSL_CTX_set_verify(SSL_CTX *ctx, int mode, int (*callback)(int, X509_STORE_CTX *));
void SSL_CTX_set_verify_depth(SSL_CTX *ctx, int depth);
/* The CAs in the PEM file CAfile; vaxpunk: no CApath. */
int SSL_CTX_load_verify_locations(SSL_CTX *ctx, const char *CAfile, const char *CApath);
/* The CAs in X509_get_default_cert_file's file, or SSL_CERT_FILE's. */
int SSL_CTX_set_default_verify_paths(SSL_CTX *ctx);
uint64_t SSL_CTX_set_options(SSL_CTX *ctx, uint64_t options);
int SSL_CTX_set_min_proto_version(SSL_CTX *ctx, int version);
int SSL_CTX_set_max_proto_version(SSL_CTX *ctx, int version);

SSL *SSL_new(SSL_CTX *ctx);
void SSL_free(SSL *ssl);
int SSL_set_fd(SSL *ssl, int fd);
/* The server's name to send, SNI. */
int SSL_set_tlsext_host_name(SSL *ssl, const char *name);
/* The name the server's certificate must have. */
int SSL_set1_host(SSL *ssl, const char *hostname);
int SSL_connect(SSL *ssl);
int SSL_read(SSL *ssl, void *buf, int num);
int SSL_write(SSL *ssl, const void *buf, int num);
int SSL_shutdown(SSL *ssl);
int SSL_get_error(const SSL *ssl, int ret);
const char *SSL_get_version(const SSL *ssl);
const SSL_CIPHER *SSL_get_current_cipher(const SSL *ssl);
const char *SSL_CIPHER_get_name(const SSL_CIPHER *cipher);
#define SSL_get_cipher(s) SSL_CIPHER_get_name(SSL_get_current_cipher(s))
long SSL_get_verify_result(const SSL *ssl);
X509 *SSL_get1_peer_certificate(const SSL *ssl);
#define SSL_get_peer_certificate SSL_get1_peer_certificate

#endif
