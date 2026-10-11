+++
title = "Run-time libraries"
band = "lang"
summary = "The shareable libraries programs call"
+++

The routines every VMS program calls besides the system services: the
general-purpose `LIB$`, `STR$`, `OTS$` and `MTH$` libraries in
`LIBRTL.EXE` and friends, screen management (`SMG$`), the callable
utilities (`SOR$`, `CONV$`, `FDL$`, `LBR$`, `CLI$`, `TPU$`, ...), the C
run-time library, threads, and each language's own library. They follow
the calling standard, take strings by descriptor and return condition
values, so any language can call any of them. vaxpunk has `LIBRTL.EXE`
with a handful of `LIB$` routines ([ADR-0028](../../docs/adr/0028-shareable-images.md)),
the `CLI$` parser linked into each image, and a small C run-time library
([ADR-0032](../../docs/adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md)).
