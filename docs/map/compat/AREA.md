+++
title = "Compatibility"
band = "lang"
summary = "Running programs built for other systems"
+++

Ways a VMS system runs software that wasn't written for it: binary
translators for older VMS images (VEST for VAX, AEST for Alpha, each
with the Translated Image Environment), PDP-11 compatibility mode on
the VAX, and the UNIX layers, POSIX for OpenVMS and then GNV, on the C
run-time library. vaxpunk adds two of its own ideas: WASMRUN, a
WebAssembly interpreter in MACRO-32 for WASI programs
([PRD-0007](../../prd/0007-wasmrun.md)), and a Linux-binary escape
hatch. Running VAX or Alpha binaries is a non-goal of Milestone 1, not
dropped.
