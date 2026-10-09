# ADR-0029 — LOGINOUT logs a terminal's process in from SYSUAF.DAT, whose passwords are salted, iterated SHA-256, and gives it its command interpreter in kernel mode

Oct 9, 2026 · @Marko Mikulicic

Proposed. Every terminal's process starts as `LOGINOUT.EXE`, with every
privilege, named after the terminal. It asks for a username and a
password, reads the user's record in `SYSUAF.DAT`, an indexed file in
VMS's `$UAFDEF` layout, and checks the password's hash, which
`$HASH_PASSWORD` makes with an algorithm of vaxpunk's: SHA-256, salted
and iterated, truncated to `UAF$Q_PWD`'s 8 bytes. Then it calls
`EXE$LOGIN` in kernel mode, which activates the record's command
interpreter in P1 and makes the process the user's, UIC, privileges and
username. The console's `LOGINOUT` is the swapper's, after a `STARTUP`
process has run `SYSTARTUP_VMS.COM`; a TELNET terminal's is `TELNETD`'s.
Step 4 of [PRD-0003](../prd/0003-multi-user-vms.md), and its open
question about the hash.

## Context

Until now every process was `SYSTEM`: `EXEC$START` created the console's
with `DCL.EXE`, whose first command was `SYSTARTUP_VMS.COM`, and
`TELNETD` created a remote login's the same way. The link address said
which image was a command interpreter
([ADR-0006](0006-cli-in-p1-runs-images-in-its-process.md)), and nothing
said who the user was.

On VMS the job controller starts `LOGINOUT` on a terminal, or the
terminal driver asks it to, at the first key. `LOGINOUT` is installed
with privileges; it reads `SYS$SYSTEM:SYSUAF.DAT` through RMS by
username, hashes the password with `$HASH_PASSWORD` and compares the
hash with `UAF$Q_PWD`, counts a wrong password in `UAF$W_LOGFAILS`,
and, in kernel mode, sets the process's UIC, privileges and quotas and
maps the command interpreter into P1, before it exits into it. VMS's
hashes are Purdy's polynomial (`UAI$C_PURDY_S`), which is broken.
`STARTUP` is a process of its own that runs the startup procedures
before anyone logs in, and ends with `SYSTEM job terminated`.

`SYSUAF.DAT` is an indexed file on OpenVMS, keyed by username, UIC and
identifiers, and RMS reads and writes indexed files now
([PRD-0008](../prd/0008-rms-record-and-indexed-files.md)).

## Decision

1. **`SYSUAF.DAT` is VMS's.** An indexed file, `vms/sysuaf.fdl`'s, of
   `$UAFDEF` records, their 644-byte fixed part (`lib.mlb`), found as
   `SYSUAF` with the default `SYS$SYSTEM:.DAT`, so a logical name `SYSUAF`
   can move it. `build.rs` writes it into the system disk's `[SYSEXE]`
   with `ods`'s loader: `SYSTEM`, password `MANAGER`, every privilege, UIC
   `[1,4]`, default `SYS$SYSROOT:[SYSMGR]`; and `DEFAULT`, the values a
   new user gets, `DISUSER`. Only `SYSTEM` may read it,
   `(S:RWE,O:RWE,G,W)`. The system disk is write locked, so
   `SYSTARTUP_VMS.COM` copies it to the ramdisk, where `SYS$SYSTEM` finds
   it first and `AUTHORIZE` can change it until the system stops.
2. **The hash is `UAI$C_SHA256`, 128, a number VMS doesn't use.**
   `$HASH_PASSWORD pwddsc, alg, salt, usrdsc, hash`, VMS's service, takes
   it alone: SHA-256 of the salt, a word, the username without its blanks
   and the password, then 4095 times of the last digest and the password,
   and the first 8 bytes of the last digest. It is MACRO-32 in the
   executive (`hashpwd.mar`), and Rust on the host for `build.rs`
   (`vms/uafhash.rs`), which a known answer test checks against FIPS
   180-4's. Passwords are in capitals, as VMS's are.
3. **`LOGINOUT` runs on each terminal, in a process with every
   privilege, named after the terminal.** The swapper creates the
   console's once `STARTUP` is gone, and again whenever it is deleted;
   `TELNETD` creates one for each TELNET terminal. Three failures end it,
   and with it a TELNET connection. A wrong username and a wrong password
   both say `User authorization failure`; only a wrong password counts,
   which the next login reports.
4. **`EXE$LOGIN uaf` makes the process the user's**, called by `LOGINOUT`
   in kernel mode with `$CMKRNL`: it activates `UAF$T_DEFCLI`'s image,
   `DCL.EXE`, in P1, refusing one linked elsewhere, keeps `LOGINOUT`'s
   channels for it, so a TELNET terminal outlives the image, and takes the
   record's username (`PCB$T_USERNAME`, `JPI$_USERNAME`), UIC, base
   priority, authorized privileges and default ones. `LOGINOUT` defines
   `SYS$LOGIN`, `SYS$LOGIN_DEVICE`, `SYS$SCRATCH` and `SYS$DISK` and sets
   the default directory first, in user mode. When it exits, the
   executive calls DCL, as it calls a command interpreter when an image
   exits. ADR-0006's link address still says what a command interpreter
   is; the UAF now says which one a process gets.
5. **DCL starts as VMS's does.** On a terminal it runs
   `SYS$MANAGER:SYLOGIN.COM` and `SYS$LOGIN:LOGIN.COM`, each if it is
   there; when `SYS$INPUT` is a file, as `STARTUP`'s
   `SYS$MANAGER:SYSTARTUP_VMS.COM` is, it runs it as a procedure and logs
   out. `LOGOUT` writes `logged out at`, or `job terminated at` for a
   process without a terminal, and deletes the process.
6. **`SET PASSWORD` runs `SETPWD.EXE`, which the executive gives
   `SYSPRV`** while it runs, as VMS installs its own: a fixed list of
   known images in `process.mar` (`KNOWN`), each read from the system
   disk by a name with an underscore, which no logical name can redirect.
   It changes only the record of the process's username, after the old
   password.
7. **`MCR OPCCRASH` halts the system**, with `CMKRNL`, since `LOGOUT` no
   longer does.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Purdy's polynomial, so a VMS `SYSUAF.DAT` could be read | It is broken, and nothing reads our UAF but us (PRD-0003) |
| A KDF such as PBKDF2 or scrypt | HMAC and memory-hard work in MACRO-32 for no gain an 8-byte hash keeps; iterated salted SHA-256 is crypt-sha256's scheme |
| `SYSUAF.DAT` as fixed records `AUTHORIZE` rewrites, as VAX/VMS V1.0's | OpenVMS's is indexed, and RMS has indexed files now |
| `LOGINOUT` writing the PCB itself in kernel mode | It would have to know the PCB's layout and the image activator's registers; `EXE$LOGIN` keeps both in the executive |
| The terminal driver starting `LOGINOUT` at the first key, as VMS's asks the job controller to | There is no job controller yet, and the swapper creating it at once is a few lines; the console asks for a username again right after a `LOGOUT` |
| `SET PASSWORD` in `SET.EXE`, with `SYSPRV` | `SET PROTECTION` would have it too, for any file |
| Trusting a known image by its name | A process can define `SYS$SYSTEM` in its own table, and logical names have no access modes yet |
| `INSTALL /PRIVILEGED` | Installed images are after Milestone 1 (PRD-0003's tier 3) |

## Consequences

**What gets harder.**
- Every boot test and the network test log in first, and the console's
  process is the swapper's to restart, not `SYSTEM`'s to end the system.
- `SYSUAF.DAT` on the ramdisk is lost at each boot, with every user
  `AUTHORIZE` added, unless a site defines `SYSUAF` on the data disk.
- Two processes can have `SYSUAF.DAT` open at once, and RMS doesn't lock
  its buckets yet: two updates at the same moment can lose one. ponytail:
  RMS's sharing, PRD-0003's step 7.

**What stays easy.**
- A user is a record: `AUTHORIZE ADD` from `DEFAULT`'s, and `LOGINOUT`
  reads it with the same RMS calls as any program.
- Changing the hash means a new `UAI$C_` number in `UAF$B_ENCRYPT`;
  records with the old one keep working while `$HASH_PASSWORD` knows it.

**Follow-ups:** quotas from the record (step 5); `LGICMD`, account
restrictions, password expiry, last login times; `LNM$JOB` and
`LNM$GROUP`; the terminal driver asking for a login at a key; a
`SHUTDOWN.COM` before `OPCCRASH`; `INSTALL` in place of `KNOWN`.
