/* openssl/x509.h: certificates, their names, and the results of checking
 * them. */
#ifndef OPENSSL_X509_H
#define OPENSSL_X509_H

typedef struct x509_st X509;
typedef struct X509_name_st X509_NAME;
typedef struct x509_store_ctx_st X509_STORE_CTX;

#define X509_V_OK 0
#define X509_V_ERR_UNSPECIFIED 1
#define X509_V_ERR_UNABLE_TO_GET_ISSUER_CERT 2
#define X509_V_ERR_CERT_SIGNATURE_FAILURE 7
#define X509_V_ERR_CERT_NOT_YET_VALID 9
#define X509_V_ERR_CERT_HAS_EXPIRED 10
#define X509_V_ERR_DEPTH_ZERO_SELF_SIGNED_CERT 18
#define X509_V_ERR_UNABLE_TO_GET_ISSUER_CERT_LOCALLY 20
#define X509_V_ERR_CERT_CHAIN_TOO_LONG 22
#define X509_V_ERR_CERT_REVOKED 23
#define X509_V_ERR_INVALID_PURPOSE 26
#define X509_V_ERR_CERT_REJECTED 28
#define X509_V_ERR_HOSTNAME_MISMATCH 62

X509_NAME *X509_get_subject_name(const X509 *x);
X509_NAME *X509_get_issuer_name(const X509 *x);
/* The name as /C=US/O=Org/CN=host into the size bytes at buf, or into
 * memory OPENSSL_free frees if buf is NULL. */
char *X509_NAME_oneline(const X509_NAME *name, char *buf, int size);
void X509_free(X509 *x);
const char *X509_verify_cert_error_string(long n);
/* The CAs SSL_CTX_set_default_verify_paths loads: the file the logical
 * name SSL_CERT_FILE names, else SSL3$CERTS:CERT.PEM. vaxpunk: OpenSSL's
 * is SSL3$ROOT:[000000]cert.pem, which needs a rooted logical name. */
const char *X509_get_default_cert_file(void);
const char *X509_get_default_cert_file_env(void);

#endif
