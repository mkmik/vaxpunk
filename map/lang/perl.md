+++
title = "Perl"
summary = "Perl 5, which has built on VMS from its own sources since the 1990s"
aka = ["Perl", "Perl 5", "PERL", "VMS::Stdio", "VMS::DCLsym", "VMS::Filespec"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "//lang/c"
why = "Perl is C"

[[needs]]
on = "//rtl/crtl"
why = "Files, processes and sockets through the C library"

[[needs]]
on = "//dcl/spawn"
why = "system and backticks run DCL commands in subprocesses"

[[needs]]
on = "//files/ods5"
why = "Module names need long, mixed-case file names"
soft = true

[[needs]]
on = "//layered/mms"
why = "Perl builds with MMS or MMK"
soft = true

[[needs]]
fuzzy = "Floating point in the C path, and a far larger C run-time library"
why = "What a Perl port asks of the system"
+++

Perl 5 builds on VMS from its own source tree, with a `README.vms`, a
DCL configuration procedure and an MMS description file, and modules
for VMS's own facilities: `VMS::Stdio` for RMS options, `VMS::DCLsym`
for DCL symbols, `VMS::Filespec` to turn VMS file names into UNIX ones.
OpenSSL's build on VMS needs it, among others. It runs on the C
run-time library and spawns subprocesses for `system`.

vaxpunk has none. ADR-0032 counts needing Perl to configure it among
the reasons it put SSL3 on Mbed TLS rather than OpenSSL.
