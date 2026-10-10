/*
 * SSL3's libcrypto: OpenSSL's error queue, certificates and names, on Mbed
 * TLS; and what Mbed TLS asks of the platform: entropy from
 * $GET_ENTROPY, the time from the C run-time library.
 */
#include <starlet.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <mbedtls/error.h>
#include <mbedtls/platform.h>
#include <mbedtls/platform_time.h>
#include <mbedtls/platform_util.h>
#include <mbedtls/x509_crt.h>
#include <openssl/crypto.h>
#include <openssl/err.h>
#include <openssl/x509.h>
#include "ssl3.h"

const char *OpenSSL_version(int type)
{
	(void)type;
	return OPENSSL_VERSION_TEXT;
}

unsigned long OpenSSL_version_num(void)
{
	return OPENSSL_VERSION_NUMBER;
}

/* The error queue: the oldest first, the newest dropped when it's full. */
#define ERRORS 16
static unsigned long errors[ERRORS];
static int nerrors;

void ssl3$push_error(unsigned long e)
{
	if (nerrors < ERRORS)
		errors[nerrors++] = e;
}

void ssl3$push_mbedtls(int ret)
{
	ssl3$push_error(ret == MBEDTLS_ERR_X509_CERT_VERIFY_FAILED
				? ERR_PACK(ERR_LIB_SSL, 0, SSL_R_CERTIFICATE_VERIFY_FAILED)
				: ERR_PACK(ERR_LIB_SSL, 0, ERR_R_MBEDTLS | -ret));
}

unsigned long ERR_get_error(void)
{
	unsigned long e = nerrors ? errors[0] : 0;

	if (nerrors)
		memmove(errors, errors + 1, --nerrors * sizeof errors[0]);
	return e;
}

unsigned long ERR_peek_error(void)
{
	return nerrors ? errors[0] : 0;
}

void ERR_clear_error(void)
{
	nerrors = 0;
}

const char *ERR_reason_error_string(unsigned long e)
{
	static char text[100];
	int reason = ERR_GET_REASON(e);

	if (reason & ERR_R_MBEDTLS) {
		mbedtls_strerror(-(reason & ~ERR_R_MBEDTLS), text, sizeof text);
		return text;
	}
	switch (reason) {
	case SSL_R_CERTIFICATE_VERIFY_FAILED: return "certificate verify failed";
	case SSL_R_UNEXPECTED_EOF_WHILE_READING: return "unexpected eof while reading";
	}
	return NULL;
}

void ERR_error_string_n(unsigned long e, char *buf, size_t len)
{
	const char *reason = ERR_reason_error_string(e);

	snprintf(buf, len, "error:%08lX:%s::%s", e,
		 ERR_GET_LIB(e) == ERR_LIB_SSL ? "SSL routines" : "",
		 reason ? reason : "");
}

char *ERR_error_string(unsigned long e, char *buf)
{
	static char text[256];

	if (!buf)
		buf = text;
	ERR_error_string_n(e, buf, 256);
	return buf;
}

void ERR_print_errors_fp(FILE *fp)
{
	char text[256];
	unsigned long e;

	while ((e = ERR_get_error())) {
		ERR_error_string_n(e, text, sizeof text);
		fprintf(fp, "%s\n", text);
	}
}

X509_NAME *X509_get_subject_name(const X509 *x)
{
	return (X509_NAME *)&x->crt.subject;
}

X509_NAME *X509_get_issuer_name(const X509 *x)
{
	return (X509_NAME *)&x->crt.issuer;
}

/* Mbed TLS writes C=US, O=Org, CN=host, with a backslash before a comma
 * in a value; OpenSSL's one line is /C=US/O=Org/CN=host. */
char *X509_NAME_oneline(const X509_NAME *name, char *buf, int size)
{
	char text[512], *d;
	const char *s;
	int len;

	len = mbedtls_x509_dn_gets(text, sizeof text, (const mbedtls_x509_name *)name);
	if (len < 0)
		return NULL;
	if (!buf) {
		size = len + 2;
		buf = malloc(size);
		if (!buf)
			return NULL;
	}
	if (size <= 0)
		return buf;
	for (s = text, d = buf; *s && d < buf + size - 1; s++) {
		if (s == text)
			*d++ = '/';
		if (s > text && s[0] == ',' && s[1] == ' ' && s[-1] != '\\') {
			*d++ = '/';
			s++;
		} else {
			*d++ = *s;
		}
	}
	*d = 0;
	return buf;
}

void X509_free(X509 *x)
{
	if (x) {
		mbedtls_x509_crt_free(&x->crt);
		free(x);
	}
}

const char *X509_verify_cert_error_string(long n)
{
	switch (n) {
	case X509_V_OK: return "ok";
	case X509_V_ERR_UNABLE_TO_GET_ISSUER_CERT: return "unable to get issuer certificate";
	case X509_V_ERR_CERT_SIGNATURE_FAILURE: return "certificate signature failure";
	case X509_V_ERR_CERT_NOT_YET_VALID: return "certificate is not yet valid";
	case X509_V_ERR_CERT_HAS_EXPIRED: return "certificate has expired";
	case X509_V_ERR_UNABLE_TO_GET_ISSUER_CERT_LOCALLY: return "unable to get local issuer certificate";
	case X509_V_ERR_CERT_CHAIN_TOO_LONG: return "certificate chain too long";
	case X509_V_ERR_CERT_REVOKED: return "certificate revoked";
	case X509_V_ERR_INVALID_PURPOSE: return "unsupported certificate purpose";
	case X509_V_ERR_CERT_REJECTED: return "certificate rejected";
	case X509_V_ERR_HOSTNAME_MISMATCH: return "hostname mismatch";
	}
	return "unspecified certificate verification error";
}

const char *X509_get_default_cert_file(void)
{
	return "SSL3$CERTS:CERT.PEM";
}

const char *X509_get_default_cert_file_env(void)
{
	return "SSL_CERT_FILE";
}

/* Mbed TLS's verification flags as OpenSSL's verification result: the
 * first problem, in OpenSSL's order of checking. */
long ssl3$verify_result(uint32_t flags)
{
	static const struct {
		uint32_t flag;
		long result;
	} map[] = {
		{ MBEDTLS_X509_BADCERT_NOT_TRUSTED, X509_V_ERR_UNABLE_TO_GET_ISSUER_CERT_LOCALLY },
		{ MBEDTLS_X509_BADCERT_FUTURE, X509_V_ERR_CERT_NOT_YET_VALID },
		{ MBEDTLS_X509_BADCERT_EXPIRED, X509_V_ERR_CERT_HAS_EXPIRED },
		{ MBEDTLS_X509_BADCERT_REVOKED, X509_V_ERR_CERT_REVOKED },
		{ MBEDTLS_X509_BADCERT_KEY_USAGE, X509_V_ERR_INVALID_PURPOSE },
		{ MBEDTLS_X509_BADCERT_EXT_KEY_USAGE, X509_V_ERR_INVALID_PURPOSE },
		{ MBEDTLS_X509_BADCERT_CN_MISMATCH, X509_V_ERR_HOSTNAME_MISMATCH },
		{ MBEDTLS_X509_BADCERT_BAD_MD, X509_V_ERR_CERT_REJECTED },
		{ MBEDTLS_X509_BADCERT_BAD_PK, X509_V_ERR_CERT_REJECTED },
		{ MBEDTLS_X509_BADCERT_BAD_KEY, X509_V_ERR_CERT_REJECTED },
		{ MBEDTLS_X509_BADCERT_OTHER, X509_V_ERR_CERT_CHAIN_TOO_LONG },
	};

	if (flags == 0)
		return X509_V_OK;
	for (size_t k = 0; k < sizeof map / sizeof map[0]; k++)
		if (flags & map[k].flag)
			return map[k].result;
	return X509_V_ERR_UNSPECIFIED;
}

/* Mbed TLS's entropy: $GET_ENTROPY's, 256 bytes at a time, and none if
 * it fails, so that no key comes from less (ADR-0031). */
int mbedtls_platform_get_entropy(psa_driver_get_entropy_flags_t flags, size_t *estimate_bits,
				 unsigned char *output, size_t output_size)
{
	if (flags)
		return PSA_ERROR_NOT_SUPPORTED;
	for (size_t k = 0; k < output_size; k += 256) {
		size_t n = output_size - k < 256 ? output_size - k : 256;
		if (!(sys$get_entropy(output + k, n) & 1))
			return PSA_ERROR_INSUFFICIENT_ENTROPY;
	}
	*estimate_bits = 8 * output_size;
	return 0;
}

mbedtls_ms_time_t mbedtls_ms_time(void)
{
	struct timeval tv;

	gettimeofday(&tv, NULL);
	return (mbedtls_ms_time_t)tv.tv_sec * 1000 + tv.tv_usec / 1000;
}

struct tm *mbedtls_platform_gmtime_r(const mbedtls_time_t *tt, struct tm *tm_buf)
{
	time_t t = *tt;

	return gmtime_r(&t, tm_buf);
}
