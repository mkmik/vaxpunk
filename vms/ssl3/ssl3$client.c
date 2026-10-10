/*
 * SSL3$CLIENT: a TLS client, the way a C program on OpenVMS uses SSL3.
 *
 *     $ SSLCLIENT :== $SYS$SYSTEM:SSL3$CLIENT
 *     $ SSLCLIENT host port [cafile]
 *
 * Connects to port on host, by name or address, and makes a TLS
 * connection that checks the server's certificate against the CAs in
 * cafile, or else in SSL_CERT_FILE or SSL3$CERTS:CERT.PEM, and its name
 * against host.
 * Says what it made, then sends what is typed, a line at a time with
 * CR LF after each, until an empty line or CTRL/Z, and shows what the
 * server sends back until it closes.
 */
#include <errno.h>
#include <inet.h>
#include <netdb.h>
#include <socket.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <openssl/err.h>
#include <openssl/ssl.h>

static int fail(const char *what)
{
	fprintf(stderr, "%%SSL3-E-FAIL, %s\n", what);
	ERR_print_errors_fp(stderr);
	return EXIT_FAILURE;
}

int main(int argc, char **argv)
{
	struct sockaddr_in addr = { .sin_family = AF_INET };
	struct hostent *host;
	SSL_CTX *ctx;
	SSL *ssl;
	X509 *cert;
	char line[512], *name;
	int s, n;

	if (argc < 3) {
		fprintf(stderr, "usage: sslclient host port [cafile]\n");
		return EXIT_FAILURE;
	}
	host = gethostbyname(argv[1]);
	if (!host)
		return fail("no such host");
	memcpy(&addr.sin_addr, host->h_addr, sizeof addr.sin_addr);
	addr.sin_port = htons(atoi(argv[2]));
	s = socket(AF_INET, SOCK_STREAM, 0);
	if (s < 0 || connect(s, (struct sockaddr *)&addr, sizeof addr) < 0) {
		perror("connect");
		return EXIT_FAILURE;
	}
	printf("Connected to %s, port %s\n", inet_ntoa(addr.sin_addr), argv[2]);

	SSL_library_init();
	SSL_load_error_strings();
	ctx = SSL_CTX_new(TLS_client_method());
	if (!ctx)
		return fail("SSL_CTX_new");
	if (argc > 3 ? !SSL_CTX_load_verify_locations(ctx, argv[3], NULL)
		     : !SSL_CTX_set_default_verify_paths(ctx))
		return fail("no CA certificates");
	SSL_CTX_set_verify(ctx, SSL_VERIFY_PEER, NULL);
	ssl = SSL_new(ctx);
	if (!ssl || !SSL_set_fd(ssl, s) || !SSL_set1_host(ssl, argv[1]))
		return fail("SSL_new");
	if (SSL_connect(ssl) != 1) {
		long v = SSL_get_verify_result(ssl);
		if (v != X509_V_OK)
			fprintf(stderr, "%%SSL3-E-VERIFY, %s\n", X509_verify_cert_error_string(v));
		return fail("SSL_connect");
	}
	printf("%s connection using %s\n", SSL_get_version(ssl), SSL_get_cipher(ssl));
	cert = SSL_get1_peer_certificate(ssl);
	if (cert) {
		name = X509_NAME_oneline(X509_get_subject_name(cert), NULL, 0);
		printf("Server certificate subject: %s\n", name);
		OPENSSL_free(name);
		name = X509_NAME_oneline(X509_get_issuer_name(cert), NULL, 0);
		printf("Server certificate issuer: %s\n", name);
		OPENSSL_free(name);
		X509_free(cert);
	}

	while (fgets(line, sizeof line - 1, stdin) && line[0] != '\n') {
		n = strlen(line) - 1;
		line[n++] = '\r';
		line[n++] = '\n';
		if (SSL_write(ssl, line, n) <= 0)
			return fail("SSL_write");
	}
	while ((n = SSL_read(ssl, line, sizeof line)) > 0)
		for (int k = 0; k < n; k++)
			if (line[k] != '\r')
				putchar(line[k]);
	if (n < 0 && SSL_get_error(ssl, n) != SSL_ERROR_ZERO_RETURN)
		return fail("SSL_read");
	SSL_shutdown(ssl);
	SSL_free(ssl);
	SSL_CTX_free(ctx);
	close(s);
	return EXIT_SUCCESS;
}
