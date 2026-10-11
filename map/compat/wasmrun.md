+++
title = "WASMRUN"
summary = "A WebAssembly interpreter in MACRO-32 that runs WASI Rust and Go programs"
aka = ["WASM", "WASI", "wasm32-wasip1", "WASMRUN.EXE", "WASM RUN"]
kind = "glue"
intent = "goal"
research = "outline"
refs = ["docs/prd/0007-wasmrun.md"]

[[feature]]
id = "m0-toolchain"
title = "M0: the toolchain and system can hold it: FP, quadword built-ins, memory, mapped images"
status = "todo"
needs = ["//host/vmacro", "//host/vasm", "//platform/pal", "//exec/image-activator", "//exec/timers", "//exec/memory:more-memory"]
note = "PRD-0007 M0 and Feasibility: FP in vasm and vmacro, IEEE built-ins, the 1,024-page limit (PRD-0003 tier 4 #39), image sections mapped page by page (tier 3 #21), $GETUTC, multi-module images in build.rs"

[[feature]]
id = "m1-loader"
title = "M1: loader and validator, translation to word-code; WASM VALIDATE"
status = "todo"
needs = [":m0-toolchain", "//rms/core"]
note = "PRD-0007 M1"

[[feature]]
id = "m2-integer"
title = "M2: the integer core: i32 and i64, control flow, calls, memory, tables, traps"
status = "todo"
needs = [":m1-loader", "//exec/condition-handling"]
note = "PRD-0007 M2; 64/64 division by raw sdiv/udiv, explicit division traps"

[[feature]]
id = "m3-floats"
title = "M3: f32 and f64, natively through IEEE built-ins and by soft-float"
status = "todo"
needs = [":m2-integer"]
note = "PRD-0007 M3; blocked on FP in vasm and vmacro"

[[feature]]
id = "m4-rust-hello"
title = "M4: minimal WASI: a Rust hello world and an args echo match wasmtime"
status = "todo"
needs = [":m3-floats", "//exec/entropy", "//drivers/terminal", "//rtl/librtl", "//exec/information-services"]
note = "PRD-0007 M4: args, env, streams, clocks, random_get on $GET_ENTROPY, exit; JPI$_CPUTIM missing"

[[feature]]
id = "m5-go-hello"
title = "M5: Go hello world, goroutines and time.Sleep, a panic"
status = "todo"
needs = [":m4-rust-hello", "//exec/event-flags", "//exec/asts", "//files/ods2"]
note = "PRD-0007 M5: poll_oneoff on $SETIMR and $WFLOR; Go modules need more memory and volumes over 2 MB (PRD-0003 tier 3 #22)"

[[feature]]
id = "m6-files"
title = "M6: files and directories: preopens, path mapping, versions, record formats"
status = "todo"
needs = [":m5-go-hello", "//rms/core", "//rms/sequential", "//rms/wildcards"]
note = "PRD-0007 M6: needs Stream_LF read and write, $TRUNCATE, $RENAME, $FLUSH, wildcard directories (PRD-0003 tier 3 #23)"

[[feature]]
id = "m7-dcl"
title = "M7: the WASM verb, its qualifiers, CTRL/Y, trap messages, exit statuses"
status = "todo"
needs = [":m6-files", "//dcl/cld", "//exec/messages"]
note = "PRD-0007 M7: WASM-F-* messages need message files (//utils/message), or join $GETMSG's one table"

[[feature]]
id = "m8-speed"
title = "M8: threaded dispatch, lazy translation, super-instructions"
status = "todo"
needs = [":m5-go-hello"]
note = "PRD-0007 M8: Go hello world in under 2 s under HVF"

[[feature]]
id = "m9-alpha"
title = "M9: the same sources build and pass on OpenVMS Alpha"
status = "todo"
needs = [":m7-dcl"]
note = "PRD-0007 M9: MACRO/MIGRATION on AXPbox; soft-float there"

[[feature]]
id = "vax-port"
title = "The VAX port"
status = "todo"
needs = [":m9-alpha"]
note = "PRD-0007 Later"

[[feature]]
id = "wasm-link"
title = "WASM LINK: a module bundled into an executable"
status = "todo"
needs = [":m7-dcl"]
note = "PRD-0007 v1.1"

[[feature]]
id = "sockets"
title = "WASI sockets on vaxpunk's TCP/IP"
status = "todo"
needs = [":m6-files", "//net/tcpip-qio"]
note = "PRD-0007 Later; open question: $QIO directly or the sockets library. Go's tsnet is the stated longer-term goal"

[[feature]]
id = "p2-memory"
title = "64-bit linear memory in P2 space"
status = "todo"
needs = [":m2-integer", "//exec/64-bit-addressing:p2-space"]
note = "PRD-0007 Later: the PAL maps nothing at or above 2 GB"

[[needs]]
on = "//host/vmacro"
why = "All of it is MACRO-32, and needs quadword, IEEE and dispatch support"

[[needs]]
on = "//rms/core"
why = "Reads modules and serves WASI's files through RMS"

[[needs]]
on = "//exec/memory"
why = "Linear memory grows with $CRETVA; 16-32 MB once the page limit goes"

[[needs]]
on = "//rtl/librtl"
why = "LIB$GET_FOREIGN for arguments, LIB$ file routines for WASI"

[[needs]]
on = "//exec/system-services"
why = "The host layer uses only RMS, services and the terminal driver, so it outlives seL4"
+++

WASMRUN is vaxpunk's own idea, not a VMS piece: a WebAssembly
interpreter written only in MACRO-32 that runs unmodified command-line
programs built by stock Rust (`wasm32-wasip1`) and Go (`GOOS=wasip1`).
WASI Preview 1 shrinks what a program asks of its system to about 45
host calls, which it maps onto RMS, system services and the terminal
driver, so a `.wasm` file built on the Mac runs under DCL with
`WASM RUN`, and keeps running after vaxpunk drops seL4. The same
sources are meant to build on OpenVMS Alpha and VAX too
([PRD-0007](../../docs/prd/0007-wasmrun.md)).

Nothing is built yet. PRD-0007's feasibility check found four blockers
in the system underneath: 4 MB of memory, images read whole into pool,
no floating point in vasm and vmacro, and 2 MB volumes. Its milestones
M0 to M9 are the features here; networking, threads and WASI 0.2 are
not in v1. Guests are built on the Mac with stock toolchains, and
checked byte for byte against wasmtime and the WebAssembly spec tests.
