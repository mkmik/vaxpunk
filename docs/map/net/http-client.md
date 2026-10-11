+++
title = "COPY/HTTP"
summary = "COPY from a web server, http or https, VMS's remote file syntax"
aka = ["COPY/HTTP", "COPY/FTP", "curl", "wget"]
kind = "core"
intent = "goal"
research = "done"
refs = ["vms/sysexe/copy.mar", "docs/adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md", "docs/design/0003-tcpip-port.md"]

[[feature]]
id = "http"
title = "COPY/HTTP node::\"/path\" over HTTP/1.0 into a STREAM_LF file"
status = "done"
note = "PR #110"

[[feature]]
id = "names"
title = "Hosts by name, from the hosts database or DNS"
status = "done"
needs = ["/net/hosts-db", "/net/dns-resolver#after-hosts"]

[[feature]]
id = "https"
title = "https URLs, through SSL3, checking the server's certificate"
status = "done"
needs = ["/net/ssl#macro-callable"]
note = "ADR-0032, PR #143"

[[feature]]
id = "redirects"
title = "Redirects and chunked bodies"
status = "todo"
note = "copy.mar ponytail; ADR-0032 follow-up"

[[feature]]
id = "binary"
title = "/BINARY, and wildcards"
status = "todo"
note = "copy.mar ponytail"

[[needs]]
on = "/utils/copy"
why = "It is COPY's /HTTP qualifier"

[[needs]]
on = "/net/tcpip-qio"
why = "Connects a TCP socket to the server"

[[needs]]
on = "/net/ssl"
why = "https needs TLS"
+++

TCP/IP Services had `COPY/FTP` and `COPY/RCP`, which take a remote file
as `node::"path"`. There was no `COPY/HTTP`; on VMS people ported curl
and wget, which write what they fetch as STREAM_LF files.

vaxpunk's `COPY/HTTP` is in `vms/sysexe/copy.mar`: a poor man's curl
with VMS's remote file syntax, `COPY/HTTP node::"/path"` or
`COPY/HTTP URL::"https://host/path"`. It sends an HTTP/1.0 GET on a TCP
socket and writes the body, byte for byte, into a STREAM_LF file;
`https` goes through SSL3. 404 is `%RMS-E-FNF`, 401 and 403
`%RMS-E-PRV`. It is the goal that pulled in SSL3, and through it the C
run-time library and the C cross path.
