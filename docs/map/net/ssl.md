+++
title = "SSL3 (OpenSSL API)"
summary = "TLS for programs: OpenSSL 3.0's API, here on Mbed TLS, with Mozilla's CAs"
aka = ["SSL3", "SSL1", "HP SSL", "OpenSSL", "Mbed TLS", "SSL3$LIBSSL", "SSL3$CERTS", "SSL3$CLIENT", "TLS"]
kind = "glue"
intent = "induced"
research = "done"
refs = ["docs/adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md", "vms/ssl3/ssl3$libssl.c", "vms/ssl3/ssl3$libcrypto.c", "vms/ssl3/ssl3$client.c", "vms/ssl3/config", "vms/c.rs"]

[[feature]]
id = "client"
title = "A TLS 1.2 and 1.3 client with OpenSSL's API, verifying the server"
status = "done"
note = "ADR-0032, PR #143"

[[feature]]
id = "cas"
title = "Mozilla's CA bundle as SSL3$CERTS:CERT.PEM, or SSL_CERT_FILE's file"
status = "done"
note = "ADR-0032"

[[feature]]
id = "macro-callable"
title = "MACRO-32 calls it as any routine: COPY/HTTP over https"
status = "done"
note = "ADR-0032 decision 7"

[[feature]]
id = "server"
title = "The server side: SSL_accept, certificates and keys from files"
status = "todo"
note = "ADR-0032 follow-up"

[[feature]]
id = "lazy-cas"
title = "Find the CAs a chain needs without parsing the whole bundle"
status = "todo"
note = "ADR-0032 follow-up: each connection parses 121 CAs"

[[feature]]
id = "ssl3-root"
title = "SSL3$ROOT, a rooted logical name, as VSI's kit defines"
status = "todo"
needs = ["/exec/logical-names"]
note = "ADR-0032; PRD-0003 tier 3 #25"

[[feature]]
id = "openssl-command"
title = "The OPENSSL command"
status = "todo"
note = "ADR-0032 follow-up"

[[feature]]
id = "shareable"
title = "SSL3$LIBSSL_SHR and SSL3$LIBCRYPTO_SHR shareable images"
status = "todo"
needs = ["/exec/shareable-images"]
note = "ADR-0032 decision 8: needs a shareable image that calls another"

[[needs]]
on = "/rtl/crtl"
why = "Mbed TLS and SSL3's own C need a C library: memory, strings, files, time"

[[needs]]
on = "/net/bsd-sockets"
why = "Reads and writes the socket SSL_set_fd gives it"

[[needs]]
on = "/exec/entropy"
why = "Keys come from $GET_ENTROPY, and TLS fails without it"

[[needs]]
on = "/host/cross-gcc"
why = "Mbed TLS is C, compiled by the cross gcc"

[[needs]]
on = "/host/velf"
why = "Turns gcc's objects into object modules"
+++

OpenVMS has never had TLS in the kernel or in `$QIO`. It is OpenSSL in
user mode, shipped as HP SSL and then VSI's SSL1 and SSL3 kits:
shareable images such as `SSL3$LIBSSL_SHR.EXE`, the `OPENSSL` command,
and `SSL3$STARTUP.COM`, which defines `SSL3$ROOT` and `SSL3$CERTS`.
Programs call OpenSSL's API on sockets from TCP/IP Services.

vaxpunk's SSL3 is OpenSSL 3.0's API for a TLS 1.2 and 1.3 client, in
`vms/ssl3/include/openssl`, on Mbed TLS 4.1.1, which the build
downloads, compiles with the cross gcc and converts with velf
([ADR-0032](../../adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md)).
OpenSSL itself is too large for a 4 MB machine. It exists because
COPY/HTTP wanted https, and it pulled in the C run-time library. Each
program links it statically from `SSL3$LIBSSL.OLB`; `SSL3$CLIENT` is a
TLS client in C, and `COPY/HTTP` calls it from MACRO-32.
