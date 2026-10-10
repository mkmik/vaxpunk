/*
 * SSL3's libssl: OpenSSL 3.0's connections, on Mbed TLS. An SSL_CTX keeps
 * what OpenSSL's keeps that a client needs: how to verify the server and
 * the CAs to trust. Each SSL gets an Mbed TLS configuration from it when
 * SSL_new makes it, and reads and writes its socket through the C
 * run-time library. Sockets block, so no call returns SSL_ERROR_WANT_READ
 * or SSL_ERROR_WANT_WRITE.
 *
 * Where Mbed TLS and OpenSSL differ:
 * - SSL_VERIFY_NONE still verifies, as OpenSSL does, for
 *   SSL_get_verify_result: Mbed TLS's optional verification.
 * - OpenSSL checks the server's name only after SSL_set1_host;
 *   SSL_set_tlsext_host_name only sends it. Mbed TLS checks the name it
 *   sends, so verify() forgets a mismatch when only the latter was called.
 */
#include <socket.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <mbedtls/net_sockets.h>
#include <mbedtls/ssl.h>
#include <psa/crypto.h>
#include <openssl/err.h>
#include <openssl/ssl.h>
#include "ssl3.h"

struct ssl_method_st {
	int unused;
};

struct ssl_ctx_st {
	int verify_mode;
	int verify_depth;	/* -1 for no limit */
	int min_version, max_version;
	uint64_t options;	/* SSL_OP_ */
	mbedtls_x509_crt ca;
};

struct ssl_cipher_st {
	char name[64];
};

struct ssl_st {
	SSL_CTX *ctx;
	mbedtls_ssl_config conf;
	mbedtls_ssl_context tls;
	int fd;
	int last;		/* what the last call returned, from Mbed TLS */
	int named;		/* SSL_set_tlsext_host_name or SSL_set1_host was called */
	int check_name;		/* SSL_set1_host was */
	int closed;		/* the peer's close_notify came */
	struct ssl_cipher_st cipher;
};

static const SSL_METHOD method;

int OPENSSL_init_ssl(uint64_t opts, const void *settings)
{
	(void)opts, (void)settings;
	return psa_crypto_init() == PSA_SUCCESS;
}

const SSL_METHOD *TLS_method(void)
{
	return &method;
}

const SSL_METHOD *TLS_client_method(void)
{
	return &method;
}

SSL_CTX *SSL_CTX_new(const SSL_METHOD *meth)
{
	SSL_CTX *ctx;

	if (!meth || !OPENSSL_init_ssl(0, NULL))
		return NULL;
	ctx = calloc(1, sizeof *ctx);
	if (!ctx)
		return NULL;
	ctx->verify_depth = -1;
	mbedtls_x509_crt_init(&ctx->ca);
	return ctx;
}

void SSL_CTX_free(SSL_CTX *ctx)
{
	if (ctx) {
		mbedtls_x509_crt_free(&ctx->ca);
		free(ctx);
	}
}

void SSL_CTX_set_verify(SSL_CTX *ctx, int mode, int (*callback)(int, X509_STORE_CTX *))
{
	(void)callback;
	ctx->verify_mode = mode;
}

void SSL_CTX_set_verify_depth(SSL_CTX *ctx, int depth)
{
	ctx->verify_depth = depth;
}

/* Reads CAfile whole, with a NUL after it, as Mbed TLS parses PEM, and
 * adds its certificates to those the context trusts: those it can parse,
 * as OpenSSL does, so that a bundle with a curve this build leaves out
 * still loads. */
int SSL_CTX_load_verify_locations(SSL_CTX *ctx, const char *CAfile, const char *CApath)
{
	FILE *f;
	char *pem = NULL;
	size_t len = 0, size = 0, n;
	int ret;

	if (!CAfile || CApath)
		return 0;
	f = fopen(CAfile, "r");
	if (!f) {
		ssl3$push_mbedtls(MBEDTLS_ERR_X509_FILE_IO_ERROR);
		return 0;
	}
	do {
		if (len + 1 >= size) {
			char *more = realloc(pem, size = size ? 2 * size : 4096);
			if (!more) {
				free(pem);
				fclose(f);
				return 0;
			}
			pem = more;
		}
		n = fread(pem + len, 1, size - len - 1, f);
		len += n;
	} while (n);
	fclose(f);
	pem[len] = 0;
	ret = mbedtls_x509_crt_parse(&ctx->ca, (unsigned char *)pem, len + 1);
	free(pem);
	if (ret < 0) {
		ssl3$push_mbedtls(ret);
		return 0;
	}
	return 1;
}

/* As OpenSSL's: a file that isn't there is no error, and leaves none on
 * the queue. */
int SSL_CTX_set_default_verify_paths(SSL_CTX *ctx)
{
	const char *file = getenv(X509_get_default_cert_file_env());

	if (!SSL_CTX_load_verify_locations(ctx, file ? file : X509_get_default_cert_file(), NULL))
		ERR_clear_error();
	return 1;
}

uint64_t SSL_CTX_set_options(SSL_CTX *ctx, uint64_t options)
{
	return ctx->options |= options;
}

int SSL_CTX_set_min_proto_version(SSL_CTX *ctx, int version)
{
	ctx->min_version = version;
	return 1;
}

int SSL_CTX_set_max_proto_version(SSL_CTX *ctx, int version)
{
	ctx->max_version = version;
	return 1;
}

/* Mbed TLS's callback for each certificate of the chain, the server's at
 * depth 0: OpenSSL's depth limit counts below the trust anchor, and its
 * name check needs SSL_set1_host. */
static int verify(void *p, mbedtls_x509_crt *crt, int depth, uint32_t *flags)
{
	SSL *s = p;

	(void)crt;
	if (depth == 0 && !s->check_name)
		*flags &= ~MBEDTLS_X509_BADCERT_CN_MISMATCH;
	if (s->ctx->verify_depth >= 0 && depth > s->ctx->verify_depth + 1)
		*flags |= MBEDTLS_X509_BADCERT_OTHER;
	return 0;
}

static int net_send(void *p, const unsigned char *buf, size_t len)
{
	ssize_t n = send(((SSL *)p)->fd, buf, len, 0);

	return n < 0 ? MBEDTLS_ERR_NET_SEND_FAILED : (int)n;
}

/* 0 when the peer has closed the connection. */
static int net_recv(void *p, unsigned char *buf, size_t len)
{
	ssize_t n = recv(((SSL *)p)->fd, buf, len, 0);

	return n < 0 ? MBEDTLS_ERR_NET_RECV_FAILED : (int)n;
}

SSL *SSL_new(SSL_CTX *ctx)
{
	SSL *s = calloc(1, sizeof *s);
	int ret;

	if (!s)
		return NULL;
	s->ctx = ctx;
	s->fd = -1;
	mbedtls_ssl_config_init(&s->conf);
	mbedtls_ssl_init(&s->tls);
	ret = mbedtls_ssl_config_defaults(&s->conf, MBEDTLS_SSL_IS_CLIENT, MBEDTLS_SSL_TRANSPORT_STREAM,
					  MBEDTLS_SSL_PRESET_DEFAULT);
	if (ret == 0) {
		mbedtls_ssl_conf_authmode(&s->conf, ctx->verify_mode & SSL_VERIFY_PEER
							    ? MBEDTLS_SSL_VERIFY_REQUIRED
							    : MBEDTLS_SSL_VERIFY_OPTIONAL);
		mbedtls_ssl_conf_ca_chain(&s->conf, &ctx->ca, NULL);
		if (ctx->min_version)
			mbedtls_ssl_conf_min_tls_version(&s->conf, ctx->min_version);
		if (ctx->max_version)
			mbedtls_ssl_conf_max_tls_version(&s->conf, ctx->max_version);
		ret = mbedtls_ssl_setup(&s->tls, &s->conf);
	}
	if (ret != 0) {
		ssl3$push_mbedtls(ret);
		SSL_free(s);
		return NULL;
	}
	mbedtls_ssl_set_verify(&s->tls, verify, s);
	mbedtls_ssl_set_bio(&s->tls, s, net_send, net_recv, NULL);
	return s;
}

void SSL_free(SSL *s)
{
	if (s) {
		mbedtls_ssl_free(&s->tls);
		mbedtls_ssl_config_free(&s->conf);
		free(s);
	}
}

int SSL_set_fd(SSL *s, int fd)
{
	s->fd = fd;
	return 1;
}

int SSL_set_tlsext_host_name(SSL *s, const char *name)
{
	int ret = mbedtls_ssl_set_hostname(&s->tls, name);

	if (ret != 0) {
		ssl3$push_mbedtls(ret);
		return 0;
	}
	s->named = 1;
	return 1;
}

int SSL_set1_host(SSL *s, const char *hostname)
{
	if (!SSL_set_tlsext_host_name(s, hostname))
		return 0;
	s->check_name = 1;
	return 1;
}

/* Keeps what Mbed TLS returned, for SSL_get_error, and puts an error on
 * the queue. Returns OpenSSL's result: ret if positive, else 0 for the
 * peer's close_notify and -1 for an error. */
static int result(SSL *s, int ret)
{
	s->last = ret;
	if (ret >= 0)
		return ret;
	if (ret == MBEDTLS_ERR_SSL_PEER_CLOSE_NOTIFY) {
		s->closed = 1;
		return 0;
	}
	if (ret == MBEDTLS_ERR_SSL_CONN_EOF)
		ssl3$push_error(ERR_PACK(ERR_LIB_SSL, 0, SSL_R_UNEXPECTED_EOF_WHILE_READING));
	else if (ret != MBEDTLS_ERR_NET_SEND_FAILED && ret != MBEDTLS_ERR_NET_RECV_FAILED)
		ssl3$push_mbedtls(ret);
	return -1;
}

int SSL_connect(SSL *s)
{
	int ret;

	/* No name: check none, rather than Mbed TLS's refusal to verify. */
	if (!s->named)
		mbedtls_ssl_set_hostname(&s->tls, NULL);
	do
		ret = mbedtls_ssl_handshake(&s->tls);
	while (ret == MBEDTLS_ERR_SSL_WANT_READ || ret == MBEDTLS_ERR_SSL_WANT_WRITE);
	return ret == 0 ? result(s, 1) : result(s, ret);
}

/* A TLS 1.3 server's NewSessionTicket comes as a WANT_READ with nothing
 * read: Mbed TLS takes it, and the read goes on. A close without
 * close_notify is an error, unless SSL_OP_IGNORE_UNEXPECTED_EOF makes it
 * the end, as close_notify is. */
int SSL_read(SSL *s, void *buf, int num)
{
	int ret;

	do
		ret = mbedtls_ssl_read(&s->tls, buf, num);
	while (ret == MBEDTLS_ERR_SSL_WANT_READ || ret == MBEDTLS_ERR_SSL_WANT_WRITE);
	if (ret == 0 || ret == MBEDTLS_ERR_SSL_CONN_EOF)
		ret = s->ctx->options & SSL_OP_IGNORE_UNEXPECTED_EOF ? MBEDTLS_ERR_SSL_PEER_CLOSE_NOTIFY
								     : MBEDTLS_ERR_SSL_CONN_EOF;
	return result(s, ret);
}

/* Writes all num bytes, a record at a time, as OpenSSL does unless told
 * it may write some. */
int SSL_write(SSL *s, const void *buf, int num)
{
	const unsigned char *p = buf;
	int done = 0, ret;

	while (done < num) {
		ret = mbedtls_ssl_write(&s->tls, p + done, num - done);
		if (ret == MBEDTLS_ERR_SSL_WANT_READ || ret == MBEDTLS_ERR_SSL_WANT_WRITE)
			continue;
		if (ret < 0)
			return result(s, ret);
		done += ret;
	}
	return result(s, num);
}

/* Sends close_notify. 1 if the peer's came already, else 0, as
 * OpenSSL's first call returns. */
int SSL_shutdown(SSL *s)
{
	int ret = mbedtls_ssl_close_notify(&s->tls);

	if (ret < 0 && ret != MBEDTLS_ERR_SSL_WANT_READ && ret != MBEDTLS_ERR_SSL_WANT_WRITE)
		return result(s, ret);
	return s->closed;
}

int SSL_get_error(const SSL *s, int ret)
{
	if (ret > 0)
		return SSL_ERROR_NONE;
	if (s->last == MBEDTLS_ERR_SSL_PEER_CLOSE_NOTIFY)
		return SSL_ERROR_ZERO_RETURN;
	if (s->last == MBEDTLS_ERR_NET_SEND_FAILED || s->last == MBEDTLS_ERR_NET_RECV_FAILED)
		return SSL_ERROR_SYSCALL;
	return SSL_ERROR_SSL;
}

const char *SSL_get_version(const SSL *s)
{
	/* Mbed TLS's TLSv1.2 and TLSv1.3 are OpenSSL's names. */
	return mbedtls_ssl_get_version(&s->tls);
}

/* Mbed TLS's name for the cipher suite as OpenSSL's: TLS1-3-AES-128-GCM-SHA256
 * is TLS_AES_128_GCM_SHA256, and TLS-ECDHE-RSA-WITH-AES-128-GCM-SHA256 is
 * ECDHE-RSA-AES128-GCM-SHA256. */
const SSL_CIPHER *SSL_get_current_cipher(const SSL *s)
{
	struct ssl_cipher_st *c = (struct ssl_cipher_st *)&s->cipher;
	const char *m = mbedtls_ssl_get_ciphersuite(&s->tls);
	char *d = c->name, *end = c->name + sizeof c->name - 1;

	if (!m)
		return NULL;
	if (!strncmp(m, "TLS1-3-", 7)) {
		strcpy(d, "TLS_");
		for (m += 7, d += 4; *m && d < end; m++)
			*d++ = *m == '-' ? '_' : *m;
		*d = 0;
		return c;
	}
	if (!strncmp(m, "TLS-", 4))
		m += 4;
	while (*m && d < end) {
		if (!strncmp(m, "WITH-", 5)) {
			m += 5;
		} else if (!strncmp(m, "AES-", 4)) {
			strcpy(d, "AES");
			d += 3, m += 4;
		} else if (!strcmp(m, "POLY1305-SHA256")) {
			strcpy(d, "POLY1305");
			d += 8, m += 15;
		} else {
			*d++ = *m++;
		}
	}
	*d = 0;
	return c;
}

const char *SSL_CIPHER_get_name(const SSL_CIPHER *cipher)
{
	return cipher ? cipher->name : "(NONE)";
}

long SSL_get_verify_result(const SSL *s)
{
	uint32_t flags = mbedtls_ssl_get_verify_result(&s->tls);

	return flags == (uint32_t)-1 ? X509_V_ERR_UNSPECIFIED : ssl3$verify_result(flags);
}

X509 *SSL_get1_peer_certificate(const SSL *s)
{
	const mbedtls_x509_crt *peer = mbedtls_ssl_get_peer_cert(&s->tls);
	X509 *x;

	if (!peer)
		return NULL;
	x = calloc(1, sizeof *x);
	if (!x)
		return NULL;
	mbedtls_x509_crt_init(&x->crt);
	if (mbedtls_x509_crt_parse_der(&x->crt, peer->raw.p, peer->raw.len) != 0) {
		X509_free(x);
		return NULL;
	}
	return x;
}
