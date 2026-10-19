# ADR-0040 — Files keep ACLs in their headers, which the file system checks before the protection mask, and identifiers come from RIGHTSLIST.DAT through services that run in their caller's mode

Oct 19, 2026 · @Marko Mikulicic

Proposed. vaxpunk gets VMS's access control lists and rights
identifiers. `RIGHTSLIST.DAT` is VMS's indexed file of identifiers and
their holders, which `AUTHORIZE`'s identifier commands change. `LOGINOUT`
gives each process a rights list: the identifiers its user holds, and
`INTERACTIVE` and `LOCAL` or `REMOTE`. A file's ACL is a list of access
control entries in its header, where ODS-2 keeps it; the file system
reads and changes it through `IO$_ACCESS` and `IO$_MODIFY`, and checks
it before the protection mask, as VMS's XQP does. New files get ACEs
from their directory or their previous version. `SET SECURITY`, `SHOW
SECURITY`, `DIRECTORY/ACL` and `/SECURITY` and `SHOW PROCESS/RIGHTS` work
as on OpenVMS. The rights database services and the ACL services,
`$PARSE_ACL`, `$FORMAT_ACL`, `$GET_SECURITY` and `$SET_SECURITY`, run in
the mode of their caller, in the vector, through RMS and `$QIO` like any
program. `t8y:/exec/acls`, `t8y:/exec/rights-database` and
`t8y:/utils/set-show#security`.

## Context

Until now a file's owner and its protection mask were all that decided
who could use it ([PRD-0003](../prd/0003-multi-user-vms.md), steps 1 and
2). To share a file with some users but not others, the users had to be
in one UIC group. VMS solves that with identifiers: a name for a set of
users, which `AUTHORIZE GRANT/IDENTIFIER` gives to each of them, and an
ACE on the file such as `(IDENTIFIER=PAYROLL,ACCESS=READ)`. VMS V6 made
`SET SECURITY` and `SHOW SECURITY` the commands for an object's owner,
protection and ACL, through `$SET_SECURITY` and `$GET_SECURITY`.

What VMS does was taken from three places: OpenVMS Alpha V8.4-2L1 in
AXPbox (the commands' output and messages, the record layouts of
`RIGHTSLIST.DAT`, the ACEs in a header dumped byte by byte, the
definitions in `STARLET.MLB`), the VAX/VMS V4.3 sources (read for facts,
never copied: the XQP's `ACLSUBR` and `CHKPRO`, `SYSACLSRV`, the rights
database services, `SET ACL`), and VSI's DCL help.

The pieces VMS has that vaxpunk didn't: the file of identifiers, a rights
list per process, ACEs in file headers, the check, the propagation to new
files, the services and the commands. They depend on each other in that
order, so they come together.

## Decision

1. **`RIGHTSLIST.DAT` is VMS's.** An indexed file, `vms/rightslist.fdl`,
   of `$KGBDEF` records: a 48-byte record for each identifier (its value,
   attributes, a holder of 0 and its name, padded with blanks), a 16-byte
   record for each holder of one (the identifier, attributes and the
   holder's UIC), and a 64-byte maintenance record whose `KGB$L_NEXT_ID`
   is the next general identifier `$ADD_IDENT` gives out. Key 0 is the
   identifier, key 1 the holder, key 2 the name, as VMS's. `build.rs`
   writes it with the environmental identifiers `BATCH`, `DIALUP`,
   `INTERACTIVE`, `LOCAL`, `NETWORK` and `REMOTE` and a UIC identifier for
   each user in `SYSUAF.DAT`, and `SYSTARTUP_VMS.COM` copies it to the
   ramdisk next to `SYSUAF.DAT`, where `AUTHORIZE` can change it. It is
   found as `RIGHTSLIST`, `SYS$SYSTEM:.DAT` by default. Anyone may read
   it, `(S:RWED,O:RWED,G:R,W:R)`, as VMS V4.3's `$CREATE_RDB` made it.
2. **The rights database services run in their caller's mode**, in the
   vector: `$ASCTOID`, `$IDTOASC`, `$FIND_HELD`, `$FIND_HOLDER`,
   `$ADD_IDENT`, `$REM_IDENT`, `$ADD_HOLDER`, `$REM_HOLDER`,
   `$MOD_IDENT`, `$MOD_HOLDER` and `$FINISH_RDB`, with VMS's arguments
   and statuses. Each opens the file with RMS, as a program would, so
   changing it takes write access to it. `SYS$name` is the service's own
   code, with no change of mode, as `$UNWIND`'s and `$PUTMSG`'s are.
3. **A process's rights list is a block of pool** that `PCB$L_RIGHTS`
   points at: a quadword for each identifier, its value and attributes,
   as VMS's. `LOGINOUT` builds it with `$FIND_HELD` of the user's UIC,
   adds `INTERACTIVE` and `LOCAL`, or `REMOTE` on a TELNET terminal, and
   gives it to `EXE$LOGIN`. `$CREPRC` gives a copy to a process of its
   creator's UIC. `$GETJPI`'s `JPI$_PROCESS_RIGHTS` and `JPI$_RIGHTSLIST`
   return it after the UIC, and `SHOW PROCESS/RIGHTS` shows it.
4. **A file's ACL is ODS-2's**: its ACEs, `$ACEDEF`'s, one after the
   other in the header's ACL area, from `FH2$B_ACOFFSET` up to
   `FH2$B_RSOFFSET`, which they end at. The file system changes them with
   VMS's attribute codes: `ATR$C_ADDACLENT`, `DELACLENT`, `FNDACLENT`,
   `DELETEACL`, `DELETE_ALL`, `READACL`, `ACLLENGTH` and `READACE`, at
   the position `FIB$L_ACLCTX` holds, with the status in
   `FIB$L_ACL_STATUS`, by VMS's rules: an ACE goes before the one at the
   position, and replaces one that matches it, the same type and
   identifiers and both or neither of them `DEFAULT`; `DELETEACL` keeps
   the `PROTECTED` ones. Writing takes control access, reading read or
   control access.
5. **`FIL$CHKHDR` looks at the ACL first.** The first identifier ACE
   that isn't a `DEFAULT` one and whose identifiers the process all
   holds, by its UIC, with `[g,*]`, `[*,*]` and `*` wildcards, or by its
   rights list, decides. If it grants all the access asked for, that is
   the access. If it doesn't, only the system and the owner categories of
   the protection mask, and the privileges, can still give it. With no
   match, the mask decides as before.
6. **New files inherit.** A new version gets the ACL of the one before
   it, but for its `NOPROPAGATE` ACEs; a new name gets its directory's
   `DEFAULT` ACEs, without the option, and the protection of the
   directory's `DEFAULT_PROTECTION` ACE.
7. **`$PARSE_ACL`, `$FORMAT_ACL`, `$GET_SECURITY` and `$SET_SECURITY`
   run in their caller's mode too.** The first two turn an ACE's text
   into its bytes and back, with `$ASCTOID` and `$IDTOASC` for the names.
   The last two take the class `FILE`: they find the file with `$PARSE`
   and `$SEARCH` and ask the file system for its owner, protection and
   ACL with a `$QIO`, which checks the access.
8. **The commands are OpenVMS's.** `SET SECURITY` (`SET.EXE`) with
   `/ACL`, `/DELETE`, `/REPLACE`, `/AFTER`, `/LIKE`, `/DEFAULT`,
   `/OWNER`, `/PROTECTION`, `/CLASS` and `/LOG`, over wildcards; `SHOW
   SECURITY` (`SHOW.EXE`); `DIRECTORY/ACL`, `/SECURITY`, and the ACL in
   `/FULL`; owners shown by name, `[SYSTEM]`; `AUTHORIZE`'s `ADD`,
   `REMOVE`, `RENAME`, `MODIFY`, `GRANT`, `REVOKE` and `SHOW`
   `/IDENTIFIER` and `SHOW/RIGHTS`, and a UIC identifier for each user it
   adds; `SHOW PROCESS/RIGHTS`.

## Alternatives considered

| Option | Why not |
| --- | --- |
| The rights database services in kernel mode, calling RMS from there with `SYSPRV` while they read | Every argument to probe, and kernel code where a mistake stops the system, for a file anyone may read anyway; VMS's own run in executive mode, not kernel |
| `RIGHTSLIST.DAT` readable by `SYSTEM` alone, as OpenVMS V8.4's | The services run in their caller's mode, so a user's `SHOW SECURITY` couldn't name identifiers; V4.3's protection lets them |
| The identifiers in memory, loaded at boot | `AUTHORIZE` changes them while the system runs, and VMS's are a file |
| `SET SECURITY` doing the `$QIO` itself, as `SET FILE` does | `$SET_SECURITY` is what VMS's uses and what programs call; one copy of the lookup |
| A context that keeps an RMS stream open between `$IDTOASC` calls, as VMS's per-process block does | Code in the vector has no storage of its own; the context counts the records returned, and each call reads past them again |
| ACLs over extension headers | The file system has none yet: an ACL must fit beside the map, else `SS$_ACLFULL` |
| Security classes other than `FILE` | Devices have no protection yet, and the other objects no owner; `SS$_NOCLASS` until they do |

## Consequences

**What gets harder.**
- Every access check walks the file's ACL, if it has one, and its
  identifiers through the process's rights list.
- A file's header holds less map when it has an ACL, so a file with a
  long ACL and many extents can't grow: `SS$_HEADERFULL`.
- `RIGHTSLIST.DAT` on the ramdisk is lost at each boot, with
  `SYSUAF.DAT`, unless a site defines `RIGHTSLIST` on the data disk.
- A process's rights list is what its user held when it logged in:
  `GRANT/IDENTIFIER` doesn't reach a process already there, as on VMS.

**What stays easy.**
- The identifiers are records any program reads with RMS, and the ACEs
  bytes in a header `ods` can dump; both are VMS's layouts.
- Another object class, a device or a logical name table, plugs its own
  ACL into the same check and the same services.

**Follow-ups:** `SET RIGHTS_LIST` and dynamic identifiers; resource
identifiers and disk quotas; alarm and audit ACEs; `EDIT/ACL`; ACLs on
devices, queues and logical name tables; `$CHKPRO` and `$CHECK_ACCESS`;
extension headers for long ACLs.
