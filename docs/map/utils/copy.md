+++
title = "COPY"
summary = "Copies files, or appends one to another; COPY/HTTP fetches from the web"
aka = ["COPY", "APPEND", "COPY.EXE", "COPY/HTTP", "COPY/CONCATENATE", "COPY/FTP", "COPY/RCP"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["vms/sysexe/copy.mar", "vms/cld/copy.cld", "vms/cld/append.cld", "docs/adr/0009-ramdisk-writable-files-11.md", "docs/adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md", "docs/prd/0003-multi-user-vms.md", "docs/prd/0008-rms-record-and-indexed-files.md", "https://docs.vmssoftware.com/vsi-openvms-dcl-dictionary-a-m/"]

[[feature]]
id = "basic"
title = "Copy a file record by record, keeping its organization and attributes"
status = "done"
milestone = "m0"
note = "ADR-0009 decision 6"

[[feature]]
id = "append"
title = "APPEND: the records go on the end of another file"
status = "done"
milestone = "m0"

[[feature]]
id = "log"
title = "/LOG: %COPY-S-COPIED and %APPEND-S-APPENDED"
status = "done"
milestone = "m0"

[[feature]]
id = "whole-files"
title = "Relative and indexed files copied a block at a time, structure and all"
status = "done"
note = "PRD-0008: COPY of an indexed file keeps it whole"

[[feature]]
id = "http"
title = "COPY/HTTP node::\"/path\": a web server's file, over HTTP/1.0 on TCP"
status = "done"
needs = ["/net/tcpip-qio"]
note = "PR #110; the qualifier's own gaps are in t8y:/net/http-client"

[[feature]]
id = "https"
title = "https:// URLs through SSL3, checking the server's certificate"
status = "done"
needs = ["/net/ssl"]
note = "ADR-0032, PR #143: COPY calls SSL3's C routines from MACRO-32"

[[feature]]
id = "wildcards"
title = "Wildcards and lists of input files"
status = "done"
needs = ["/rms/wildcards#directories"]
note = "A file each where to leaves out its name or type, or with /NOCONCATENATE; the first error ends it"

[[feature]]
id = "long-records"
title = "Records longer than its buffer, RECSIZE"
status = "todo"
note = "copy.mar ponytail"

[[feature]]
id = "concatenate"
title = "/CONCATENATE: several inputs into one output"
status = "done"
note = "The first copied, the others appended, %COPY-S-APPENDED; to found by from's wildcard is passed over"

[[feature]]
id = "qualifiers"
title = "/CONFIRM, /REPLACE, /OVERLAY, /ALLOCATION, /PROTECTION"
status = "done"
note = "COPY does /REPLACE and /OVERLAY with $ERASE and $OPEN: RMS has no FAB$M_SUP or FAB$M_CIF; /OVERLAY only into a sequential file"

[[feature]]
id = "contiguous"
title = "/CONTIGUOUS: the new file in one run of blocks"
status = "todo"
note = "needs FAB$M_CTG in $CREATE, and the XQP to look for a run long enough"

[[feature]]
id = "selection"
title = "/BEFORE, /SINCE, /BY_OWNER, /EXCLUDE and the other file selection qualifiers"
status = "todo"

[[feature]]
id = "remote"
title = "node::file to and from another VMS"
status = "todo"
needs = ["/net/remote-files"]

[[feature]]
id = "ftp-rcp"
title = "COPY/FTP and COPY/RCP, TCP/IP Services' remote copies"
status = "todo"
note = "t8y:/net/ftp and t8y:/net/r-commands are aspirations"

[[needs]]
on = "/rms/core"
why = "$OPEN, $CREATE, and block I/O for files copied whole"

[[needs]]
on = "/rms/sequential"
why = "$GET and $PUT for each record; STREAM_LF output for COPY/HTTP"

[[needs]]
on = "/net/ssl"
why = "https:// in COPY/HTTP: the goal that pulled in SSL3, and with it the C run-time library"

[[needs]]
on = "/net/tcpip-qio"
why = "COPY/HTTP's socket, $QIO on TCPIP$DEVICE:"

[[needs]]
on = "/net/http-client"
why = "COPY/HTTP's own component: redirects, chunked bodies, /BINARY"
soft = true

[[needs]]
on = "/rtl/cli"
why = "Its parameters and qualifiers"

[[needs]]
on = "/rtl/librtl"
why = "Writes /LOG's lines with LIB$PUT_OUTPUT"

[[needs]]
on = "/net/ftp"
why = "COPY/FTP"
soft = true

[[needs]]
on = "/net/r-commands"
why = "COPY/RCP"
soft = true
+++

`COPY` makes new files from old ones: one to one, many to many with
wildcards, or several into one with `/CONCATENATE`. Where it can, VMS's
copies a file by blocks, so the copy keeps its organization, record
format and keys. `APPEND` adds a file's records to the end of another.
Over DECnet both take `node::file` on either side. TCP/IP Services added
`COPY/FTP` and `COPY/RCP` for remote copies over TCP/IP.

vaxpunk's `COPY` and `APPEND` are `vms/sysexe/copy.mar`. Each input
of the list is found with `$PARSE` and `$SEARCH`. A sequential
file is copied a `$GET` and a `$PUT` per record; a relative or indexed
file a block at a time, into a file made with the same attributes. The
new file takes the next version unless the output names one. `COPY/HTTP`
is vaxpunk's own, a poor man's curl with VMS's remote file syntax:
`COPY/HTTP node::"/path"` or `COPY/HTTP URL::"https://host/path"` writes
the body into a STREAM_LF file. Wanting `https` here is what brought in
SSL3 on Mbed TLS, and through it the C run-time library and the C cross
path ([ADR-0032](../../adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md)).
`/CONFIRM`, `/REPLACE`, `/OVERLAY`, `/ALLOCATION` and `/PROTECTION` are
VMS's. Missing: the selection qualifiers, `/CONTIGUOUS`, records longer
than its buffer, and going on to the next file after an error.
