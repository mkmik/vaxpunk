# ADR-0032 — C programs link a C run-time library with DEC C's names, and SSL3 is OpenSSL 3.0's API on Mbed TLS, which the build downloads into the user's cache with Mozilla's CAs

Oct 9, 2026 · @Marko Mikulicic

Proposed. TLS is a library in user mode, above the sockets' `$QIO`
interface, as on OpenVMS. Programs see VSI's SSL3: OpenSSL 3.0's API
in `<openssl/ssl.h>`. Under that API is Mbed TLS 4.1.1, compiled by the
cross gcc and converted by velf. Mbed TLS and SSL3's own C need a C
library, so vaxpunk starts one: the C run-time library, with DEC C's
names, header files and record-oriented stdio. It reaches VMS through a
few routines in MACRO-32 and ARM64. C programs link both statically,
from object libraries, and MACRO-32 programs call SSL3 as they call any
routine: `COPY/HTTP` fetches `https` URLs. The build downloads Mbed
TLS's release and Mozilla's CA bundle once into the user's cache and
checks their SHA-256s. The bundle goes on the system disk as
`SSL3$CERTS:CERT.PEM`, the CAs SSL3 trusts by default. `SSL3$CLIENT` is
a TLS 1.2 and 1.3 client in C that checks the server's certificate.

## Context

OpenVMS has never had TLS in the kernel or in `$QIO`. TLS there is
OpenSSL in user mode: HP's SSL, then VSI's SSL1 and SSL3 kits, with
shareable images such as `SSL3$LIBSSL_SHR.EXE` and
`SSL3$LIBCRYPTO_SHR.EXE`. Programs call OpenSSL's API and give it
sockets from TCP/IP Services. VSI's examples make a socket with
`$ASSIGN` and `$QIOW`, turn its channel into a descriptor with
`decc$socket_fd`, and pass that to `SSL_set_fd`. vaxpunk has those
sockets ([ADR-0024](0024-sockets-have-tcpip-services-qio-interface.md)),
and random bytes good enough for keys
([ADR-0031](0031-entropy-from-virtio-rng.md)).

The TLS libraries that could run here were weighed in October 2026.
OpenSSL itself is too large for a 4 MB machine, and needs a full C
library and Perl to configure it. wolfSSL is GPLv3, BearSSL has no TLS
1.3, and picotls has no X.509. rustls in user mode needs nightly Rust
for `-Zfixed-x18`, and would be 515 KB of code in each process. Mbed TLS
4.1 is Apache-2.0, an LTS release supported until March 2029. It is
written for embedded systems, and compiles unchanged with velf's gcc
flags. A TLS 1.2 and 1.3 client with X.509 is about 240 KB of code that
uses no FP or SIMD register.

PR #141 made C usable in VMS images. velf converts gcc's ELF objects into
object modules, and CDEMO shows what crosses between C's conventions and
VMS's ([DESIGN-0004](../design/0004-calling-standard.md), *C and
Fortran*). Two things don't cross. gcc sets no argument count in x9, so
a routine that reads its count, or a service whose dispatcher checks it,
can't be called from C directly. And gcc's pointers are 64-bit, where
DEC C's are 32-bit by default, so a C structure with a pointer in it
isn't VMS's layout: a descriptor or an item list is 16 bytes, not 8.

Mbed TLS needs little from a C library: `mem*`, `str*`, `calloc`,
`free`, `snprintf`, `time` and `gmtime_r`. SSL3 needs files, for the CA
certificates, and sockets. A program like VSI's examples needs `printf`,
`stdin`, `argv` and `exit` as well.

## Decision

1. **The C run-time library is DEC C's, started small.** It lives in
   `vms/crtl`, with its headers in `include/` under DEC C's names:
   `string.h`, `stdio.h`, `stdlib.h`, `time.h`, `errno.h`, `ctype.h`,
   and VMS's `starlet.h`, `descrip.h`, `ssdef.h`, `iodef.h`,
   `stsdef.h`, and TCP/IP Services' `socket.h`, `in.h`, `inet.h`,
   `netdb.h` and `tcpip$inetdef.h` (with `sys/socket.h`, `netinet/in.h`
   and `arpa/inet.h` too). Each routine is `decc$name`, as DEC C's
   `/PREFIX_LIBRARY_ENTRIES=ALL_ENTRIES` names its calls. The headers
   give each declaration that name as an assembler label. `memcpy`,
   `memset`, `memmove` and `memcmp` are there under their plain names
   as well, since gcc calls those for its own copies. It works the way
   DEC C's library does on a record-oriented system:
   - `stdout` and `stderr` gather a line and write it to `SYS$OUTPUT` as
     a record. `stdin` reads a record from `SYS$INPUT`, with what
     `stdout` has gathered as its prompt. `fopen` reads a file through
     RMS, a record at a time, each a line.
   - `exit(0)` ends the image with `SS$_NORMAL`. Any other status is
     the condition value the image ends with, and `EXIT_FAILURE` is an
     error with its message inhibited.
   - `argv` is the foreign command's line, from `LIB$GET_FOREIGN`, split
     at blanks and lowercased outside quotes.
   - `strerror(EVMSERR)` is `vaxc$errno`'s message, from `$GETMSG`.
   - `getenv` translates a logical name, the process's then the
     system's, as DEC C's does after its own few names.
   - `malloc` is a first-fit list over `$EXPREG`.
   - A socket is a channel to `TCPIP$DEVICE:`, driven with `$QIOW`, and
     its descriptor names it.

   C programs link it from `DECC$CRTL.OLB`, after their own modules.
2. **VMS's side is in two small modules.** `decc$vms.mar` has the
   routines that call RMS, `LIB$PUT_OUTPUT`, `LIB$GET_INPUT`,
   `LIB$GET_FOREIGN`, `$EXPREG`, `$GETMSG`, `$TRNLNM` and `HOST_ADDR`. Each takes
   at most eight arguments, which the prologue homes from x0-x7 whatever
   x9 holds, and names them at fixed offsets from AP. `decc$sys.m64`,
   ARM64 for vasm, has a jacket for each system service `starlet.h`
   declares. `sys$qiow` and the rest are labels for those jackets. A
   jacket sign-extends each argument's longword, sets x9 to the count,
   and branches to the service, which returns to C. `decc$main.mar` is
   a C program's transfer address. It calls `decc$$start`, a module of
   its own, which builds `argv`, calls `main` and passes its result to
   `exit`.
3. **Addresses in VMS structures are 32-bit integers.** A descriptor's
   `dsc$a_pointer` and an item list's address are `unsigned int`.
   Every address a process has is below 2 GB, so the cast loses
   nothing. `$DESCRIPTOR` makes an automatic variable only, since a
   static initializer can't cut an address to 32 bits.
4. **SSL3 is OpenSSL 3.0's API for a TLS client, on Mbed TLS.**
   `vms/ssl3/include/openssl` has `ssl.h`, `err.h`, `x509.h`,
   `crypto.h` and `opensslv.h`, with OpenSSL's names, types and
   constants. Behind them, `ssl3$libssl.c` and `ssl3$libcrypto.c`
   implement this subset:
   - initialization: `OPENSSL_init_ssl`, `SSL_library_init`;
   - contexts: `SSL_CTX_new`, `SSL_CTX_set_verify`,
     `SSL_CTX_load_verify_locations`, `SSL_CTX_set_default_verify_paths`,
     `SSL_CTX_set_options` with `SSL_OP_IGNORE_UNEXPECTED_EOF`, and the
     protocol version limits;
   - connections: `SSL_new`, `SSL_set_fd`, `SSL_set_tlsext_host_name`,
     `SSL_set1_host`, `SSL_connect`, `SSL_read`, `SSL_write`,
     `SSL_shutdown` and `SSL_get_error`;
   - the cipher, the version and the verification result;
   - the peer's certificate and its names, as `X509_NAME_oneline` gives
     them;
   - the error queue, `ERR_*`.

   Where OpenSSL and Mbed TLS differ, OpenSSL's behaviour wins.
   `SSL_VERIFY_NONE` still verifies, for `SSL_get_verify_result`. A
   certificate's name is checked only after `SSL_set1_host`. An error
   is OpenSSL's packed code, `error:0A000086:SSL routines::certificate
   verify failed`, or an Mbed TLS code with Mbed TLS's text.
   `ssl3/config` configures Mbed TLS for a TLS 1.2 and 1.3 client with
   ECDHE, AES-GCM and ChaCha20-Poly1305, and RSA and ECDSA certificates.
   Its entropy comes from `$GET_ENTROPY` and fails if that fails. Its
   time comes from the C run-time library. It uses no 128-bit division,
   so it needs nothing from libgcc. `SSL3$LIBSSL.OLB` holds SSL3's
   modules and Mbed TLS's, and a program takes only the modules it
   needs: `SSL3$CLIENT` is about 300 KB.
5. **The build downloads Mbed TLS's release into the user's cache.**
   `vms/c.rs` fetches `mbedtls-4.1.1.tar.bz2` from Mbed TLS's GitHub
   releases with `curl`, checks it against a SHA-256 in the source, and
   unpacks it with `tar` into `~/Library/Caches/vaxpunk` on macOS, or
   `$XDG_CACHE_HOME/vaxpunk` (`~/.cache/vaxpunk`) elsewhere. That is the
   cache the Justfile's web demo uses, so every worktree shares one
   copy. The release tarball has Mbed TLS's generated sources, which its
   git tags lack. Mbed TLS is compiled with `-Os`, on as many threads as
   there are CPUs, and its ELF objects are kept in `OUT_DIR` under a
   hash of the configuration and the flags. A build that changes
   neither converts them again without compiling.
6. **The CAs SSL3 trusts are Mozilla's.** `vms/c.rs` downloads the
   bundle curl extracts from Mozilla's root store, the one Linux
   distributions and their container images ship, by its date and
   SHA-256, into the same cache. The system disk has it as
   `[SSL3.CERTS]CERT.PEM`, and `SYSTARTUP_VMS.COM` defines `SSL3$CERTS`
   as `SYS$COMMON:[SSL3.CERTS]`, the name VSI's `SSL3$STARTUP.COM`
   gives the certificates' directory.
   `SSL_CTX_set_default_verify_paths` loads the file the logical name
   `SSL_CERT_FILE` names, as OpenSSL reads that environment variable,
   or else `SSL3$CERTS:CERT.PEM`, and ignores a file that isn't there,
   as OpenSSL does. vaxpunk: OpenSSL's default on VMS is
   `OSSL$DATAROOT:[000000]cert.pem`, VSI's under `SSL3$ROOT`, a rooted
   logical name, which vaxpunk doesn't have yet (PRD-0003, item 25).
   VSI's kit ships no bundle, only a demonstration CA.
7. **MACRO-32 calls SSL3 directly.** A C routine is called by the
   standard, so `CALLS` reaches `SSL_connect` as it reaches
   `LIB$PUT_OUTPUT`. C ignores x9, keeps R2-R11, which live in
   AAPCS64's saved registers, and returns an `int` in R0's low
   longword. `COPY/HTTP` makes a TLS connection this way on the
   socket it connected, with `decc$socket_fd` and `SSL_set_fd`, sends
   its GET with `SSL_write` and reads the response with `SSL_read`.
   Every program but DCL links `SSL3$LIBSSL.OLB` and `DECC$CRTL.OLB`,
   and takes from them only the modules it calls. COPY sets
   `SSL_OP_IGNORE_UNEXPECTED_EOF`, since a web server may close without
   TLS's `close_notify`, as Google's does, and a body without a length
   ends at the close, as it does without TLS.
8. **Libraries are linked in, not shared.** `DECC$SHR` and SSL3's
   shareable images would need what [ADR-0028](0028-shareable-images.md)
   leaves out: a shareable image that calls another, and data in a
   symbol vector, for `stdout` and `errno`. Until then, each C program
   gets its own copy.

## Alternatives considered

| Option | Why not |
| --- | --- |
| TLS inside the TCP/IP component, below the port | The component is scaffolding: PRD-0002 plans a native stack in the executive, and seL4 is to stay minimal. TLS was never in VMS's kernel or `$QIO` |
| rustls in user mode | Needs nightly Rust for `-Zfixed-x18` and `build-std`, and is 515 KB of code with roots, against Mbed TLS's 240 KB. Its no_std crypto provider is marked not for production |
| OpenSSL itself, as VSI ships it | Several times Mbed TLS's size, and it needs a full C library and Perl to configure it. The API is what programs need, and SSL3 gives that |
| Mbed TLS's own API, or a VMS-style one like NETLIB's `NETLIB_SSL_*` | Programs ported to VMS, and VSI's examples, call OpenSSL's API. A MACRO-callable API can come later, on top of SSL3 |
| Vendor Mbed TLS into the repository | 8 MB of third-party sources in git history |
| Mbed TLS as a git submodule | Its git tags lack the generated sources, so the build would need CMake, Python and jinja2 to make them, and nested submodules in every worktree |
| A download into `OUT_DIR` | Each worktree, and each clean build, would download it again |
| Plain names, `strlen` rather than `decc$strlen` | A program's routine named `send` or `time` would meet the library's. DEC C's prefix keeps them apart, as on VMS |
| Jackets in BLISS-64, as CDEMO's | The services' jackets would need a routine per service with nothing to say. MACRO-32 has RMS's and the services' macros, and vasm can set x9, which BLISS-64 can't from C |
| MACRO-32 jackets for every service | A MACRO-32 routine homes its arguments past the eighth only up to the count in x9, which C doesn't set, so `$QIOW`'s p3 to p6 were lost |
| Teach gcc the calling standard: x9 and sign extension | A compiler change, for a handful of calls that a jacket each covers. Worth it once C calls many routines that read their count |
| `-mabi=ilp32` for 32-bit pointers | GCC has deprecated AArch64's ILP32, and the Linux cross compiler CI uses doesn't build for it |
| The bundle from a distribution's package, Debian's or Alpine's `ca-certificates` | The same Mozilla CAs, in a `.deb` or `.apk` to unpack. curl's is one plain PEM file at a dated URL that stays |
| The bundle in the repository | Pinned by its SHA-256, it is as fixed, without 190 KB more in git for each update |
| A TLS server too | Not needed yet. Mbed TLS's server and `SSL_accept` can come later, with `SSL_CTX_use_certificate_file` and the private key |

## Consequences

**What gets harder.**
- The first build of the system disk on a machine needs the network,
  `curl`, and a `tar` that unpacks bzip2: the toolchain container gets
  `bzip2`.
- C code that ports from DEC C may assume 32-bit pointers in VMS
  structures, which here must be integers. `$DESCRIPTOR` is automatic
  only.
- No condition may reach a C frame yet (DESIGN-0004), which has no
  frame descriptor: a fault in C code isn't handled as VMS would
  handle it.
- What DEC C's library has and this one doesn't yet: `stderr` goes to
  `SYS$OUTPUT`, `fopen` reads only, `printf` has no floating point, and
  `argv[0]` isn't the image's file name. The system time is taken as
  UTC. There is no `select`, and sockets block. `EFN$C_ENF` isn't
  supported by `$QIO`, so the library waits on event flag 0.
- Each C program carries its own copy of the libraries, and so does
  COPY, which grows to 342 KB: the system disk grows from 4 MB to
  8 MB.
- Each TLS connection parses the whole bundle, 121 CAs, a few hundred
  KB of heap, though it needs one or two of them.

**What stays easy.**
- A C program reads like one written for OpenVMS: `#include <starlet.h>`,
  `sys$qiow`, `$DESCRIPTOR`, `decc$socket_fd`, `SSL_set_fd`, and status
  values from `exit`.
- More of DEC C's library is more routines in `vms/crtl`, each a C
  function under its `decc$` name, or a MACRO-32 routine where it needs
  RMS or a service.
- Mbed TLS's configuration is two headers. A newer release is a new
  name and SHA-256 in `vms/c.rs`.
- `boot/tests/network.rs` runs `SSL3$CLIENT` against rustls servers on
  the host: TLS 1.3, TLS 1.2, a certificate whose name is wrong, and a
  host that doesn't exist.

**Follow-ups:** the server side (`SSL_accept`, certificates and keys
from files); `SSL3$ROOT` once rooted logical names come; finding the
CAs a chain needs without parsing the whole bundle, with Mbed TLS's
trusted CA callback; the `OPENSSL` command; `DECC$SHR.EXE` and SSL3's
shareable images, once shareable images can call each other and export
data; `EFN$C_ENF` in `$QIO`; `SYS$TIMEZONE_DIFFERENTIAL`; and redirects
for `COPY/HTTP`.
