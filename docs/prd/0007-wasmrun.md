# PRD-0007 — WASMRUN, a WebAssembly interpreter for VMS written in MACRO-32

Oct 5, 2026 · @Marko Mikulicic

## Summary

WASMRUN is a WebAssembly interpreter for VMS, written only in MACRO-32, that runs unmodified command-line programs built by Rust (`wasm32-wasip1`) and Go (`GOOS=wasip1 GOARCH=wasm`). The first target is vaxpunk on ARM64. The same sources should also build on VAX, Alpha, Itanium and x86-64 OpenVMS. (WASMRUN is a working name.)

Why it matters:

- **Porting is the bottleneck.** Modern software assumes POSIX. WebAssembly plus WASI Preview 1 shrinks that assumption to about 45 host calls, which is a small, fixed surface to implement on VMS once.
- **One binary, many VMSes.** A `.wasm` file built on the Mac runs on any VMS that has WASMRUN, with no cross-compiler per platform.
- **It lives entirely in VMS userland.** It uses only RMS, system services and the terminal driver, so it keeps working after vaxpunk drops seL4.
- **It fits the era.** MACRO-32 is the period-correct language, and a large, performance-sensitive MACRO-32 program is a strong test corpus for the vaxpunk MACRO-32 → ARM64 compiler.

Not in v1: networking, threads, the component model (WASI 0.2), JIT compilation. Networking is the obvious next step, since running Go programs such as tsnet is a stated longer-term goal.

## Goals and non-goals

v1 is done when a Rust and a Go command-line program, built with stock toolchains on the Mac, read files, write files, print output and exit with the right status under DCL on vaxpunk.

**Goals**

1. Run modules built by stock `rustc` (target `wasm32-wasip1`) and stock Go (`GOOS=wasip1`), with no custom flags or patched runtimes.
2. Implement the full WebAssembly 2.0 core instruction set except SIMD, plus every WASI Preview 1 function (unsupported ones return an error code, never a missing import).
3. Pass the official WebAssembly spec test suite for the supported features.
4. Write 100% of the interpreter and host layer in MACRO-32. No C, BLISS or Rust in the shipped image.
5. Keep one source tree that builds on vaxpunk (ARM64) and on at least one legacy VMS (Alpha first, then VAX).
6. Behave like a normal VMS citizen: DCL invocation, VMS file specs where the user types them, condition values on failure, `$STATUS` set on exit, quotas respected.
7. Fail safely. A bad or malicious module can only trap; it can never corrupt the interpreter or touch memory outside its own linear memory.

**Non-goals for v1**

- Networking (`sock_*` calls return `ENOSYS`/`ENOTSUP`).
- Threads, shared memory, atomics.
- SIMD (`v128`), exception handling, tail calls, GC types, memory64, multiple memories.
- WASI 0.2 / the component model (`wasm32-wasip2`).
- A JIT or ahead-of-time compiler to native code.
- Speed competitive with native code. Correctness first, then a simple fast path.
- Embedding WASMRUN as a callable library for other VMS programs (keep the door open, do not build it).

## Target platforms and portability

One MACRO-32 source tree, with platform differences pushed into a small set of macros chosen at build time. Three things differ between platforms and drive most of the design: how 64-bit integers are done, how IEEE floats are done, and where a large linear memory can live.

| Platform | MACRO-32 toolchain | 64-bit integers (`i64`) | IEEE floats (`f32`/`f64`) | Linear memory home | Priority |
| --- | --- | --- | --- | --- | --- |
| vaxpunk (ARM64) | Marko's MACRO-32 → ARM64 cross compiler | Native, through quadword built-ins the compiler provides | Native, through IEEE built-ins the compiler provides; soft-float as fallback | 32-bit space first; 64-bit region later | v1 |
| OpenVMS Alpha | `MACRO/MIGRATION` (AMACRO compiler) | Native through `EVAX_` quadword built-ins | Soft-float unless the `EVAX_` built-ins cover IEEE (to verify) | P0 (≤ 1 GB); P2 via 64-bit services later | v1.x |
| OpenVMS VAX | VAX MACRO assembler | Emulated with longword pairs (`ADWC`, `EMUL`, a 64/64 divide routine) | Soft-float only (VAX has no IEEE hardware) | P0 (≤ 1 GB) | v2 |
| OpenVMS Itanium / x86-64 | `MACRO/MIGRATION` (IMACRO / x86 compiler) | Native through `EVAX_` built-ins (they carry over) | Soft-float first | P0, P2 later | Best effort |

**Portability rules for the source**

- Write only the MACRO-32 subset the compiled-MACRO compilers accept: declared routine entries (`.CALL_ENTRY` / `.JSB_ENTRY`, with a small compatibility macro file for VAX), no branching into other routines, no self-modifying code, no reliance on condition codes across calls, no tricks with the return address on the stack.
- Never touch a platform-specific instruction outside the platform macro files: `I64.MAR` (64-bit arithmetic), `FLT.MAR` (float), `MEM.MAR` (linear memory access), `DISPATCH.MAR` (opcode dispatch).
- All four platforms are little-endian, like WebAssembly, so loads and stores need no byte swapping.
- Unaligned access: WebAssembly allows it. It is cheap on ARM64 and VAX, but traps to a slow OS fixup on Alpha. The memory macros must split possibly-unaligned accesses into byte or aligned pieces on Alpha.

**What vaxpunk needs from its own toolchain** (cross-project requirements):

- Quadword built-ins in the MACRO-32 → ARM64 compiler, ideally with the same names and meaning as Alpha's `EVAX_` set (`EVAX_ADDQ`, `EVAX_MULQ`, `EVAX_UMULH`, `EVAX_LDQ`, `EVAX_STQ`, shifts, compares).
- IEEE single and double built-ins (add, sub, mul, div, sqrt, compare, convert, min/max, rounding modes) mapped to ARM64 FP instructions.
- Good code for computed jumps and `CASEL` tables, since opcode dispatch is the hot path.
- A user-mode RMS subset and terminal `$QIO` (see the WASI host layer section).

## Guest toolchains and the WASI version

The "system" both compilers target is **WASI Preview 1**: the module imports its host functions from a module named `wasi_snapshot_preview1`. That is the only interface WASMRUN must provide in v1.

| Language | Build command | Notes |
| --- | --- | --- |
| Rust | `cargo build --target wasm32-wasip1 --release` | Target was called `wasm32-wasi` before the rename. Full `std` works: files, args, env, time, stdin/stdout. Use `panic = "abort"` for smaller modules. |
| Go | `GOOS=wasip1 GOARCH=wasm go build -o app.wasm` | Supported since Go 1.21. Modules are large (a hello-world is a few MB) and very call- and `i64`-heavy. |
| C | `clang --target=wasm32-wasip1` with wasi-libc / wasi-sdk | Not a goal, but it comes for free and is useful for small tests. |
| TinyGo | `tinygo build -target=wasip1` | Optional; much smaller Go modules, handy for early bring-up. |

**Not targeted:** `wasm32-wasip2` / WASI 0.2 (needs the component model, a much bigger runtime), `wasm32-unknown-unknown` (no system interface at all), and `GOOS=js` (needs a JavaScript host).

**What these toolchains actually emit**, which sets the feature list in the next section:

- Recent Rust enables by default: sign-extension ops, mutable globals, multi-value, reference types (mainly a changed encoding of the table index in `call_indirect`), bulk memory (`memory.copy`, `memory.fill`) and non-trapping float-to-int conversions.
- Go uses wasm globals as its own registers (stack pointer, goroutine pointer), unwinds and rewinds the wasm call stack to switch goroutines, and puts a large `br_table` at the start of many functions to resume them. So `call`, `return`, `br_table` and `global.get/set` must be fast.
- Go's runtime calls `random_get` and `clock_time_get` during start-up and uses `poll_oneoff` to sleep and to wait on file descriptors. These must work before "hello world" can print.

Pin the exact Rust and Go versions used for acceptance tests, and re-check the emitted feature set with `wasm-tools validate --features` when bumping them.

## WebAssembly feature scope

v1 supports WebAssembly 2.0 core without SIMD, binary format only. A module that uses anything outside this list is rejected at load time with a clear message naming the feature, never at run time.

| Feature | v1 | Why |
| --- | --- | --- |
| MVP core (i32, i64, f32, f64, control flow, one memory, one table) | Yes | Baseline |
| Sign-extension operators | Yes | Emitted by Rust and Go |
| Non-trapping float-to-int (`trunc_sat`) | Yes | Emitted by recent Rust; cheap |
| Multi-value (blocks and functions returning several values) | Yes | Rust default |
| Mutable global import/export | Yes | Rust/Go stack pointer globals |
| Bulk memory (`memory.copy/fill/init`, `data.drop`, table ops) | Yes | Rust default; big speed-up for `memcpy` |
| Reference types (`funcref`/`externref`, several tables, `table.get/set/grow`) | Yes | Rust default; small to implement |
| Text format (`.wat`) | No | Convert on the Mac with `wasm-tools` |
| SIMD (`v128`) | No | Not enabled by default in Rust or Go |
| Threads, atomics, shared memory | No | No threads in v1 |
| Exception handling, tail calls, GC, memory64, multi-memory, extended const | No | Not emitted by the target toolchains today |

**Exact semantics that are easy to get wrong** (each needs targeted tests):

- Integer division: trap on divide by zero, and on `INT_MIN / -1` for signed division; `rem_s` of `INT_MIN % -1` returns 0 and does not trap.
- Shift counts are taken modulo 32 or 64. VAX `ASHL` uses a signed count and different limits, so shifts cannot map straight onto it.
- Float NaNs: arithmetic may return any NaN, but `abs`, `neg`, `copysign` and loads/stores must be pure bit operations that keep the NaN payload. Reinterpret ops are bit copies.
- `min`/`max`: NaN in, NaN out; `-0` is less than `+0`.
- Float to int (`trunc`) traps on NaN or out of range; `trunc_sat` clamps instead.
- Rounding: all arithmetic round-to-nearest-even; `nearest` is ties-to-even, not ties-away.
- Memory: every access checks `address + offset + size ≤ current size` in unsigned 33-bit (or 65-bit) arithmetic, so the add cannot wrap.
- Data and element segments are applied at instantiation in order; an out-of-bounds segment fails instantiation with a trap.

## Architecture

WASMRUN is one VMS image with four parts: a load path, a run loop, a WASI host layer, and a thin platform macro layer that is the only code that changes between ports.

A module goes through the loader and translator once; the run loop then executes word-code and calls into the host layer, which uses only standard VMS interfaces.

Proposed source modules: `LOADER.MAR`, `VALIDATE.MAR`, `XLATE.MAR`, `RUN.MAR`, `WASI_*.MAR` (one per WASI group), the four platform files, `WASMMSG.MSG` (message facility) and `WASM.CLD` (DCL verb).

## Interpreter design

The interpreter validates each function once at load time and translates it into an internal "word-code": fixed-width opcodes with operands already decoded and every branch target already resolved. The run loop never decodes LEB128 or searches for the end of a block.

**Load-time pass (validate + translate)**

- Full validation per the spec, done in the same pass as translation. The translator relies on validated invariants (stack heights, types), so skipping validation is not an option.
- Output per instruction: one longword opcode index, then fixed longword or quadword operands (local index, memory offset, constant, branch target as a word-code offset, and how many stack values a branch keeps and drops).
- `br_table` becomes an inline table of (target, keep, drop) entries. Go modules have tables with thousands of entries; translation must stay linear in size.
- Translation is lazy per function: done on first call. Go modules have thousands of functions and many are never called, so this cuts start-up time.
- Later, as an optimisation: fuse common pairs into super-instructions (`local.get` + `i32.add`, compare + `br_if`), and cache the top of stack in a register.

**Run-time state** (pinned in registers inside the run loop routine):

| Register role | Holds |
| --- | --- |
| Instruction pointer | Next word-code longword |
| Value stack pointer | Top of the operand stack |
| Frame pointer | Base of the current function's locals |
| Memory base | Start of linear memory |
| Memory limit | Current linear memory size in bytes |
| Instance pointer | Globals, tables, function table, host-call table |

**Value representation:** every stack slot and local is 8 bytes. An `i32` uses the low longword; `f32` and `f64` are stored as raw IEEE bits; a `funcref` is a function index or a null marker. Uniform slots keep branches and calls simple.

**Calls:** wasm calls do not use the VMS call stack. The interpreter keeps its own call-frame stack (return word-code address, frame base, function index), so wasm recursion depth is limited by a configurable size, not by the process's native stack, and a trap can unwind it in one step. Host (WASI) calls are normal `CALLS` into MACRO-32 routines.

**Dispatch:** chosen per platform in `DISPATCH.MAR`:

- Portable: `CASEL` on the opcode index. The compiled-MACRO compilers turn this into a jump table.
- Threaded: the word-code stores handler addresses instead of indexes, and each handler ends with an indirect jump to the next one (`JMP @(Rn)+` on VAX). Use where the compiler handles computed jumps well, which should include vaxpunk; whether Alpha's AMACRO accepts this needs checking.

**Linear memory:**

- Reserved as one contiguous region at instantiation, sized to the module's declared maximum or the `/MAX_MEMORY` cap, whichever is smaller. `memory.grow` only raises the limit register and zero-fills nothing (fresh pages are demand-zero).
- Every access is bounds-checked in code. No guard-page tricks in v1: a 32-bit address space cannot reserve the 4 GB + guard region that trick needs.
- An access violation inside linear-memory code is still caught by a condition handler and turned into a trap, as a safety net.
- Go modules may declare no maximum. Default cap: 256 MB, raised with `/MAX_MEMORY`. Reserving it costs page-file quota (`PGFLQUOTA`) and virtual page count, so the loader must check quotas and fail with a clear message.

**Traps:** each trap kind has its own condition value in a `WASM` message facility (`WASM-F-UNREACHABLE`, `WASM-F-INTDIV`, `WASM-F-INTOVF`, `WASM-F-OOBMEM`, `WASM-F-OOBTAB`, `WASM-F-SIGMISMATCH`, `WASM-F-STKOVF`, `WASM-F-BADCONV`). The trap message shows the function index and name (from the `name` section if present) and the byte offset in the original module, so the fault can be found with `wasm-tools` on the Mac.

## WASI host layer mapped to VMS

Every `wasi_snapshot_preview1` function is present, because instantiation fails if any import is missing and Go's `syscall` package imports nearly all of them. Each one is either implemented or returns a WASI error code (`ENOSYS` or `ENOTSUP`). The layer uses only RMS, system services, the terminal driver and run-time library calls that exist on every VMS, so it ports unchanged.

| WASI group | Functions | VMS mapping | v1 |
| --- | --- | --- | --- |
| Arguments | `args_get`, `args_sizes_get` | `LIB$GET_FOREIGN` or the CLI parse, then split with C-runtime-style quoting rules; `argv[0]` = the module name | Full |
| Environment | `environ_get`, `environ_sizes_get` | Only what the user asks for: `/ENVIRONMENT=("K=V",...)` plus logical names in a dedicated table (e.g. `WASM$ENV`) | Full |
| Exit | `proc_exit` | Flush output, then `SYS$EXIT` with a status that keeps the numeric code (see below) | Full |
| Clocks | `clock_time_get`, `clock_res_get` | Realtime: `SYS$GETUTC` (or `SYS$GETTIM` adjusted by `SYS$TIMEZONE_DIFFERENTIAL`), converted from the 1858 VMS epoch to Unix nanoseconds. Monotonic: realtime clamped so it never goes backwards. CPU time: `$GETJPI` `JPI$_CPUTIM` | Full |
| Random | `random_get` | Pluggable entropy source: `$GET_ENTROPY`, which the PAL serves from virtio-rng ([ADR-0031](../adr/0031-entropy-from-virtio-rng.md)); on legacy VMS a ChaCha20 generator seeded from time, PID and system counters, flagged as not cryptographic | Full |
| Standard streams | `fd_read`, `fd_write` on fds 0–2 | `SYS$INPUT`, `SYS$OUTPUT`, `SYS$ERROR`; terminal via `$QIO`, files via RMS (details below) | Full |
| Files | `path_open`, `fd_read`, `fd_write`, `fd_pread`, `fd_pwrite`, `fd_seek`, `fd_tell`, `fd_close`, `fd_sync`, `fd_datasync`, `fd_filestat_get`, `fd_filestat_set_size` | RMS (`$OPEN`, `$CREATE`, `$CONNECT`, block-mode `$READ`/`$WRITE`, `$GET`/`$PUT` for record files, `$FLUSH`, `$TRUNCATE`) | Full |
| Directories | `fd_readdir`, `path_create_directory`, `path_remove_directory`, `path_filestat_get` | `$SEARCH`/`$PARSE` with wildcards, `LIB$CREATE_DIR`, `LIB$DELETE_FILE` on `.DIR;1` | Full |
| Rename / delete | `path_rename`, `path_unlink_file` | `LIB$RENAME_FILE`, `LIB$DELETE_FILE` | Full |
| Preopens | `fd_prestat_get`, `fd_prestat_dir_name` | Directories granted with `/DIRECTORY` (see below) | Full |
| Descriptor flags | `fd_fdstat_get`, `fd_fdstat_set_flags`, `fd_renumber`, `fd_advise`, `fd_allocate` | Report file type (terminal = character device, mailbox = pipe-like, disk = regular file); accept append and non-blocking flags; `fd_advise` is a no-op | Full |
| Waiting | `poll_oneoff`, `sched_yield` | Clock subscriptions with `$SETIMR` + `$WFLOR`; read/write subscriptions on terminals and mailboxes with an async `$QIO` and event flags; disk files are always ready | Full |
| Times | `fd_filestat_set_times`, `path_filestat_set_times` | Revision date via the file's ACP attributes | Best effort |
| Links | `path_link`, `path_symlink`, `path_readlink` | ODS-5 symlinks where the platform supports them; otherwise `ENOTSUP` | Stub |
| Signals | `proc_raise` | `ENOSYS` | Stub |
| Sockets | `sock_accept`, `sock_recv`, `sock_send`, `sock_shutdown` | `ENOSYS` until a networking release | Stub |

**Paths and preopens.** A WASI program sees POSIX-style paths relative to a preopened directory. WASMRUN maps them to VMS file specs:

- `/DIRECTORY=("/"=DKA0:[USER.MARKO], "/tmp"=SYS$SCRATCH:)` grants directories under guest names. Default: the current default directory, preopened as `.`. Nothing else is reachable, and `..` cannot climb above a preopen.
- `a/b/c.txt` becomes `[.A.B]C.TXT` under the preopen. A name with no dot becomes `NAME.` on disk and maps back without the dot.
- On ODS-5, case is preserved and characters ODS-2 forbids (extra dots, spaces, `+`, and so on) are written with ODS-5 `^` escapes. On ODS-2, names are upper-cased and an impossible name returns `EINVAL` or `ENAMETOOLONG`. ODS-5 is recommended.
- Directory names that contain dots (`foo.d/`) use ODS-5 escapes (`[.FOO^.D]`).

**File versions.** Default policy, configurable per run:

- Open for reading: the highest version.
- Create or open with truncate: a new version (the VMS way; keeps history and is safe).
- Delete: all versions, so that delete-then-check-exists behaves the way POSIX programs expect.
- `fd_readdir` lists each name once (highest version only). A `/VERSIONS` option can expose `name;N` for tools that want it.

**Record formats.** WASI expects a stream of bytes; VMS files have record formats.

- New files are created as Stream\_LF, the same default as the VMS C runtime, so Unix-style text and binary data round-trip exactly.
- Stream\_LF, Stream, undefined and fixed-512 files are read and written with block I/O: full random access, exact size, exact `fd_seek`.
- Variable-length and VFC files (most DCL-created text files) are readable as text: each record is returned followed by a `\n`. They are sequential only; `fd_seek` other than rewind returns `ESPIPE`, and the reported size is an estimate. Writing to them returns `ENOTSUP` in v1.

**Terminals and output files.**

- Terminal output goes through `$QIO` `IO$_WRITEVBLK`, with `\n` turned into CR LF so lines do not stair-step.
- Terminal input is line-mode `$QIO` `IO$_READVBLK` with echo; the line terminator is returned as `\n`; Ctrl-Z at the start of a line is end of file.
- When `SYS$OUTPUT` is a file or batch log, output is split on `\n` into RMS records and partial lines are buffered until the next `\n`, an explicit sync, or exit.
- Ctrl-Y / Ctrl-C ends the run cleanly: the interpreter checks an AST-set flag at loop back-edges and calls, then flushes and exits with `SS$_CONTROLC` or `SS$_CONTROLY`.

**Exit status.** `proc_exit(0)` → `SS$_NORMAL`. A non-zero code `n` → a `WASM` facility status with error severity that carries `n` in its message number field, so `$STATUS` both fails in DCL and keeps the program's code (the same idea as the C runtime's POSIX exit mode). A trap → the matching `WASM-F-*` status, with a fatal severity.

## User interface: running a module from DCL

Users run a module with a `WASM` DCL verb (defined by a CLD file), and can wrap any module as its own foreign command so it feels like a native tool.

```
$ WASM RUN HELLO.WASM "world" "--count=3"
$ WASM RUN /DIRECTORY=("/"=[.DATA]) /MAX_MEMORY=512 GREP.WASM "-i" "needle" "/notes.txt"
$ GREP :== $WASM$ROOT:[BIN]WASMRUN.EXE WASM$ROOT:[APPS]GREP.WASM
$ GREP "-i" "needle" "notes.txt"
```

**Qualifiers**

| Qualifier | Default | Meaning |
| --- | --- | --- |
| `/DIRECTORY=("guest"=vms-dir, ...)` | Current default directory as `.` | Directories the program may see |
| `/ENVIRONMENT=("K=V", ...)` | Empty | Environment variables |
| `/MAX_MEMORY=n` (MB) | 256 | Cap on linear memory |
| `/CALL_DEPTH=n` | 10 000 frames | Cap on wasm call depth |
| `/VALUE_STACK=n` (KB) | 1024 | Operand stack size |
| `/[NO]CASE_LOWER` | `/NOCASE_LOWER` | Lower-case unquoted arguments that DCL upper-cased |
| `/VERSION_POLICY=keyword` | as in the host-layer section | File version behaviour |
| `/TRACE=(CALLS, HOST, TRAPS)` | Off | Log wasm calls, WASI calls and traps to `SYS$ERROR` |
| `/STATISTICS` | Off | Print load time, run time, instructions executed and peak memory at exit |
| `WASM VALIDATE file` | — | Validate and translate only; report errors and unsupported features |

**Argument handling.** DCL upper-cases unquoted words, so quoted arguments are the norm. The split follows the C runtime's rules (double quotes group words; `""` is a literal quote) so behaviour matches what VMS users already know. Unix-style options such as `-i` work when quoted. `/CASE_LOWER` is the escape hatch for typing without quotes.

**Bundled images (v1.1).** `WASM LINK GREP.WASM /EXECUTABLE=GREP.EXE` produces a normal executable that contains the interpreter and the module, so a tool can be installed with no `WASM$ROOT` dependency.

## Build, testing and conformance

Correctness is proven by the official spec tests plus side-by-side comparison with wasmtime on the Mac: same module, same input, same output and exit code.

**Build**

- vaxpunk: cross-build on the Mac with the vaxpunk MACRO-32 compiler and linker, producing a VMS `EXE`; run it in the vaxpunk QEMU image.
- Alpha: `MACRO/MIGRATION` + `LINK` on a real or emulated Alpha (e.g. AXPbox/ES40 emulator, or a hobbyist VSI license).
- VAX: `MACRO` + `LINK` on SIMH VAX.
- One build script per platform, all reading the same `.MAR` list and choosing the platform macro files.

**Test layers**

1. **Spec tests.** The WebAssembly spec suite (`.wast` files) is converted on the Mac with `wast2json` into binary modules plus a command list. A small MACRO-32 driver inside VMS runs each command (instantiate, invoke, assert return, assert trap, assert invalid) and reports pass/fail. Target: 100% of tests for supported features.
2. **Instruction unit tests.** Generated tables of edge cases for every numeric op (NaNs, ±0, ±infinity, `INT_MIN`, shift counts ≥ width, every rounding case), with expected results produced by wasmtime. Most important for the soft-float and longword-pair `i64` paths.
3. **WASI tests.** The `wasi-testsuite` programs (Rust and C) with their expected stdout and exit codes, plus WASMRUN-specific tests for VMS mapping: versions, record formats, ODS-2 vs ODS-5 names, terminal vs file output.
4. **Program tests.** A fixed set of Rust and Go programs, compared byte for byte with wasmtime: hello world, `cat`, `wc`, `grep`-like search, a JSON pretty-printer, a program that sleeps and prints the time, a recursive directory walk, a deliberate panic, a non-zero exit.
5. **Fuzzing.** `wasm-smith` generates random valid modules; run them in WASMRUN and wasmtime and compare results and trap kinds. Also feed truncated and corrupted binaries: the loader must reject them, never crash.

**Cross-platform check:** the same test run on vaxpunk and on Alpha must give identical results. Any difference is a bug in a platform macro file.

**Performance tracking.** `/STATISTICS` output from a fixed benchmark set (CoreMark compiled to wasm, a Go program that sorts 1 million integers, Go and Rust hello world start-up) recorded on every milestone.

## Milestones

Each milestone ends with something runnable on vaxpunk; the order front-loads the riskiest parts (64-bit arithmetic, floats, Go start-up).

1. **M0 — Toolchain readiness.** vaxpunk's MACRO-32 compiler accepts the needed subset, plus quadword and IEEE built-ins. Exit: a 200-line MACRO-32 test of `EVAX_`-style ops and IEEE ops passes under QEMU.
2. **M1 — Loader and validator.** Parse every section, validate, translate to word-code; `WASM VALIDATE` works. Exit: validates every module in the spec suite and the Go and Rust hello-world modules with the right accept/reject results.
3. **M2 — Integer core.** All `i32`/`i64` ops, control flow, calls, locals, globals, memory, tables, traps. Exit: integer spec tests pass; a hand-written module that calls a stub `fd_write` prints text.
4. **M3 — Floats.** Soft-float library and native IEEE path behind `FLT.MAR`. Exit: all float spec tests pass on both paths.
5. **M4 — Minimal WASI: Rust hello world.** Args, env, standard streams, clocks, random, exit, plus stubs for everything else. Exit: Rust `println!` program and `std::env::args` echo program match wasmtime.
6. **M5 — Go hello world.** `poll_oneoff`, descriptor flags, everything the Go runtime touches at start-up. Exit: Go hello world, a goroutine + `time.Sleep` program, and a panic test match wasmtime.
7. **M6 — Files and directories.** Preopens, path mapping, versions, record formats, readdir, rename, delete. Exit: Rust and Go `cat`, `wc`, directory walk and file-copy programs pass on ODS-5 and ODS-2 volumes; `wasi-testsuite` filesystem tests pass.
8. **M7 — DCL polish.** CLD verb, all qualifiers, Ctrl-Y handling, trap messages, exit statuses, `/STATISTICS`. Exit: documented in a VMS-style help library entry.
9. **M8 — Speed pass.** Threaded dispatch where supported, lazy translation, super-instructions, top-of-stack caching. Exit: Go hello world starts in under 2 s on vaxpunk under QEMU on an M3 (target, to be revised once M5 gives a baseline).
10. **M9 — Alpha port.** Exit: full test suite passes on Alpha with identical results.
11. **Later.** VAX port, bundled executables (`WASM LINK`), WASI sockets on the vaxpunk TCP/IP stack, 64-bit linear memory in P2 space.

## Feasibility on vaxpunk

Checked against `main` at 116558b (Oct 6, 2026), after
[PRD-0005](0005-macro32-on-the-calling-standard.md) put MACRO-32 on the
calling standard. The interpreter itself fits what `vmacro` compiles
today. The system underneath does not yet hold it: four limits block
WASMRUN outright, and a few more decide the milestone order. They are
WASMRUN's prerequisites, kept here rather than in PRD-0003's backlog,
though several are already items there for other reasons.

**Blockers**

| Limit | Where | What it blocks | Way out |
| --- | --- | --- | --- |
| The machine has 1,024 pages, 4 MB, and about 3.3 MB are free after boot | `PFN_COUNT` in `pal/src/main.c`; PRD-0003 item 39 | Everything: the 256 MB default is about 75 times what is free | Frame caps in a CNode of their own. The 256 MB untyped the PAL retypes from then puts the ceiling near 64-128 MB |
| The image activator reads the whole image file into nonpaged pool, which is 512 KB for the whole system | `FIL$OPENFILE` in `f11.mar`; `POOL_PAGES` in `memory.mar` | WASMRUN.EXE: 20,000-40,000 lines of MACRO-32 is likely 300-600 KB of code. `edit.mar`, the largest program today, is 44 KB of source | Map an image's sections page by page, as the `ponytail:` note there says |
| No floating point: `vmacro` has no F/D/G instructions or IEEE built-ins, and `vasm` encodes no FP arithmetic, so raw ARM64 can't stand in | `crosstools/vtools/docs/macro32.md`, `crosstools/vtools/docs/assembler.md` | M3 | FP encodings in `vasm`, then IEEE built-ins in `vmacro`. Cheaper than soft-float on ARM64 |
| A Files-11 volume holds at most 4,096 blocks, 2 MB | PRD-0003 item 22 | Go modules, several MB each, can't be stored | Clusters and multi-block bitmaps |

**What decides the order**

- **M4 and M5** have most of what they need. Terminal `$QIO` with event
  flags and ASTs, CTRL/C and CTRL/Y ASTs, `$SETIMR`, `$WFLOR`, `$HIBER`,
  `$EXIT` with any status, and handlers that catch `SS$_ACCVIO` all work.
  Missing: `$GETUTC`; `$GETTIM` serves, since system time is the RTC's
  and in effect UTC. Also missing: `JPI$_CPUTIM`, and read with timeout,
  for which `$SETIMR` and `$CANCEL` stand in. Random bytes come from
  `$GET_ENTROPY` ([ADR-0031](../adr/0031-entropy-from-virtio-rng.md)).
  The clock ticks every 10 ms.
- **M6** needs the most RMS work. `$GET` reads VAR and FIX records only,
  and sequentially. `$PUT` only appends. There is no block I/O, `$UPDATE`,
  `$TRUNCATE`, `$RENAME` or `$FLUSH`, and no wildcard directories in
  `$SEARCH` (PRD-0003 item 23). A Stream\_LF file can be created but not
  read or written. Volumes are ODS-2 only, 39.39 upper-case names, so the
  ODS-2 path mapping is the one that matters, not the fallback. Versions
  and subdirectories work, the latter through `$CREATE_DIR`.
- **M7**: CLD verbs (`vms/cld/`) and foreign commands
  (`LIB$GET_FOREIGN`) work. There is no MESSAGE compiler, and `$GETMSG`
  reads one table built into the executive, so `WASM-F-*` would print as
  `%NONAME-F-NOMSG` unless WASMRUN formats its own messages or the
  facility joins that table. A DCL line is at most 255 bytes.
- **Building it:** `vms/build.rs` makes one image per `sysexe/*.mar`;
  WASMRUN's modules need a special case like DCL's.

**The interpreter and the toolchain**

- `EVAX_ADDQ`, `SUBQ`, `MULQ`, `UMULH`, `LDQ`, `STQ`, the shifts, compares
  and conditional moves exist under Alpha's names, so `I64.MAR` is
  mostly a list of them.
- There is no `EVAX_DIVQ`; Alpha had none either. Raw `sdiv` and `udiv`
  between `.DISABLE FLAGGING` and `.ENABLE FLAGGING` do 64/64 division,
  and the register is sign-extended again before VAX instructions read it.
  Division by zero gives 0 and nothing traps, so wasm's two division traps
  are explicit checks.
- `CASEL` is a jump table, 7 instructions per dispatch, with no limit on
  its size. Its entries are 16-bit displacements, though, so each handler
  must start within 32 KB of the table: point the entries at `BRW` stubs,
  or keep a table of `.ADDRESS` entries and dispatch with
  `MOVL TAB[R0], R1` and `JMP (R1)`. `JMP @TAB[R0]` keeps VAX's meaning
  and is not a table dispatch.
- `JMP (Rn)` and `JSB (Rn)` compile to `br` and `blr`, so threaded
  dispatch works on vaxpunk.
- Condition codes are computed only when a branch reads them: `ADDL2` is
  2 instructions, `CMPL` and `BEQL` together are 2.
- `.MACRO`, `.REPT` and `.IRP` are enough to generate handler tables.
- Unaligned loads and stores work in user mode, and seL4 keeps each
  thread's FP registers, so FP state survives context switches once the
  toolchain can emit it.
- The user stack is a fixed 16 KB with no guard page, which the design
  already allows for by keeping wasm's stacks in memory of its own.

**Assumptions to change for vaxpunk**

- Pages are allocated and zeroed when they are created; nothing is
  demand-zero. Reserving the declared maximum would take that much RAM.
  Instead, linear memory starts at a fixed P0 address, which leaves
  about 1 GB of room, and `memory.grow` creates the new pages with
  `$CRETVA`.
- vaxpunk enforces no quotas, `PGFLQUOTA` included. A process that asks
  for too much gets `SS$_INSFMEM`.
- There is no P2: the PAL maps nothing at or above 2 GB.
- `/MAX_MEMORY` should default to what the machine has, 16-32 MB once
  the page limit is lifted, and `/VALUE_STACK` to 256 KB.
- CI runs QEMU with TCG. Only `just boot` on the Mac uses HVF, so M8's
  2 s target holds only under HVF. A CPU-bound process gets 200 ms
  quanta at priority 4, and the console stays usable meanwhile.

**Milestones in this light**

- M0 grows to include vaxpunk's side: the page limit, mapped image
  sections, FP in `vasm` and `vmacro`, `$GETUTC` and multi-module images. Bigger volumes and RMS block I/O and Stream\_LF
  come before M6.
- A Rust hello world needs about 1 MB of linear memory and fits even in
  today's 4 MB. TinyGo or Rust can carry M4 and M5 while Go waits for the
  page limit and bigger volumes.

**Alpha.** AMACRO has no IEEE built-ins; its float instructions are VAX F,
D and G. f32 and f64 on Alpha therefore mean soft-float. Alpha has no
divide instruction either, so `I64.MAR` needs a 64/64 divide routine
there. Whether AMACRO compiles `JMP @(Rn)+` is still open; the AXPbox
emulator can test it.

**Not measured.** The code size and instruction counts above come from
reading `vmacro`, not from running it. Go's start-up cost in wasm
instructions is unknown. An M2 benchmark replaces both guesses.

## Risks and open questions

The biggest risk is speed on Go programs: Go wasm is `i64`- and call-heavy, so a slow 64-bit path or slow calls make start-up take seconds.

**Risks**

| Risk | Effect | Mitigation |
| --- | --- | --- |
| `i64` emulated with longword pairs is slow | Go programs crawl on VAX, and on vaxpunk if built-ins are missing | Make quadword built-ins an M0 requirement for vaxpunk; accept VAX as slow |
| Soft-float bugs | Wrong numbers, hard to spot | Exhaustive generated tests against wasmtime; Go's GC pacer and `strconv` exercise floats early |
| MACRO-32 at this size is hard to maintain | Bugs, slow progress | Strict macro layers, generated handler tables, a style guide, the BLISS-style linter idea applied to MACRO-32 |
| 32-bit address space limits linear memory | Large programs fail to start | 256 MB default, clear quota errors, P2 memory as a later milestone |
| Go runtime behaviour changes between versions | A new Go release breaks start-up | Pin Go versions in acceptance tests; test new releases before bumping |
| Alpha AMACRO limits on computed jumps or IEEE built-ins | Slower or blocked Alpha port | Keep `CASEL` dispatch and soft-float as working defaults |
| Record-format mismatches | Programs see odd sizes or fail to seek on DCL-made text files | Create Stream\_LF; read VAR as text; document the limits |

**Open questions**

- [ ] Delete policy: remove all versions (POSIX-like, proposed default) or only the highest (closer to VMS habit)?
- [ ] Should the environment also expose DCL symbols, or only an explicit logical-name table?
- [ ] Is `JMP @(Rn)+` threaded dispatch accepted and fast with Alpha's AMACRO compiler?
- [x] Do Alpha's `EVAX_` built-ins include IEEE T/S-floating arithmetic, or is soft-float the only route on Alpha from MACRO-32? No; soft-float (see *Feasibility on vaxpunk*).
- [x] Where does entropy come from on vaxpunk: a new system service, or a device read via `$QIO`? A system service, VSI's `$GET_ENTROPY`, which the PAL serves from virtio-rng ([ADR-0031](../adr/0031-entropy-from-virtio-rng.md)).
- [ ] Which user-mode RMS subset will vaxpunk have by M6, and does WASMRUN drive the schedule for it?
- [ ] Should the bundled-executable format (`WASM LINK`) store the module translated (faster start) or as plain `.wasm` (simpler, portable)?
- [ ] For networking later: map `sock_*` onto the vaxpunk QIO network device directly, or onto its sockets library?
