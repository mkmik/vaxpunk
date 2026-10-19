# How a program asks the Files-11 XQP for attributes, version limits and volume changes

On VMS a program changes a file's owner, protection, dates, record
attributes or version limit with a `$QIO` to the disk: `IO$_ACCESS` reads
them and `IO$_MODIFY` writes them, with a FIB that names the file and a
list of attribute descriptors. `SET FILE`, `SET PROTECTION` and
`SET DIRECTORY` do exactly that. `SET VOLUME` is different: it opens
`INDEXF.SYS` and rewrites the home block's copies itself. This report
collects the layouts, the rules and the messages vaxpunk needs to copy
those interfaces, for [ADR-0039](../adr/0039-xqp-qio-functions.md) and
the `SET FILE` work after it (`t8y:/utils/set-show#set-file`,
`t8y:/files/ods2#version-limits`, `t8y:/files/volumes#set-volume`).

Sources: the VAX/VMS V4.3 sources (DEC's, proprietary: read for facts,
never copied): the `$FIBDEF`, `$ATRDEF`, `$FATDEF` and `$HM2DEF`
definitions, the XQP's `MODIFY`, `RWATTR`, `FIND`, `ENTER`, `GETFIB`,
`ACCESS` and `DELETE` modules, the FDT routines in `SYSACPFDT`, and SET's
`SETFILE`, `SETDIR` and `SETVOLUME`; the VSI DCL help for `SET FILE`,
`SET DIRECTORY` and `SET VOLUME`; FreeVMS's headers for the constants
later VMS added (October 2026).

## The FIB

p1 of every XQP function is a descriptor of a FIB. Offsets in bytes:

| Offset | Field | Meaning |
| --- | --- | --- |
| 0 | `FIB$L_ACCTL` | How the file is accessed (`FIB$V_WRITE` 8, `NOWRITE` 0, `NOREAD` 10, `NORECORD` 21, ...) |
| 4 | `FIB$W_FID` | The file's ID: number, sequence, then `FIB$B_FID_RVN` 8 and `FIB$B_FID_NMX` 9 |
| 10 | `FIB$W_DID` | Its directory's ID, the same shape; 0 for none |
| 16 | `FIB$L_WCC` | A wildcard lookup's context |
| 20 | `FIB$W_NMCTL` | How the name is looked up: `WILD` 8, `NEWVER` 9, `SUPERSEDE` 10, `FINDFID` 11, and the outputs `LOWVER` 14, `HIGHVER` 15 |
| 22 | `FIB$W_EXCTL` | Extend and truncate control; `FIB$W_CNTRLFUNC` for `IO$_ACPCONTROL` |
| 24 | `FIB$L_EXSZ` | Blocks to extend by; `FIB$L_CNTRLVAL` |
| 28 | `FIB$L_EXVBN` | |
| 32 | `FIB$B_ALOPTS`, `FIB$B_ALALIGN`, `FIB$W_ALLOC` | Placement |
| 44 | `FIB$W_VERLIMIT` | The name's version limit |
| 46 | `FIB$B_AGENT_MODE` | |
| 48-63 | ACL context and status, `FIB$L_STATUS`, `FIB$L_ALT_ACCESS` | |

`FIB$C_LENGTH` is 64 in V4.3; FreeVMS has 72 and 96 for later VMS.
`FIB$C_ACCDATA`, 10, is the shortest FIB a request takes: shorter is
`SS$_INSFARG`. The XQP extends a short FIB with zeros and writes it back.

The file is found three ways. With a directory ID, the XQP looks p2's
name up in it and puts the file's ID in the FIB. With `FIB$M_FINDFID` it
looks for the entry of the FIB's file ID instead, with no p2. With no
directory ID it takes the file ID as it is. A lookup returns the name's
version limit in `FIB$W_VERLIMIT`.

## The attribute list

p5 points to descriptors of 8 bytes, `ATR$W_SIZE`, `ATR$W_TYPE` and
`ATR$L_ADDR`, and a longword of 0 after the last; the FDT takes at most
30. An unknown code, or a size above the attribute's, is
`SS$_BADATTRIB`.

| Code | Name | Size | Writing it |
| --- | --- | --- | --- |
| 3 | `UCHAR` | 4 | allowed; `CONTIG`, `SPOOL`, `BADBLOCK`, `NOCHARGE` and `MARKDEL` are put back after |
| 4 | `RECATTR` | 32 | allowed; the allocation, `HIBLK`, is put back after |
| 8 | `EXPDAT` | 7, ASCII | control access |
| 9 | `STATBLK` | 32 | read only |
| 10 | `HEADER` | 512 | read only |
| 13 | `ASCDATES` | 35 | control access |
| 17-20 | `CREDATE`, `REVDATE`, `EXPDATE`, `BAKDATE` | 8 each | control access |
| 21 | `UIC` | 4 | control access |
| 22 | `FPRO` | 2 | control access |
| 26 | `UIC_RO` | 4 | read only |
| 41 | `HIGHWATER` | 4 | read only |
| 46 | `FILE_SPEC` | 512 | read only |

A write of a read-only attribute is skipped, not an error. There is no
attribute for the version limit.

The record attributes, `$FATDEF`, 32 bytes: `RTYPE` 0 (the record format
in bits 0-3: UDF 0, FIX 1, VAR 2, VFC 3, STM 4, STMLF 5, STMCR 6; the
organization in bits 4-7: SEQ 0, REL 1, IDX 2), `RATTRIB` 1 (FTN 0, CR 1,
PRN 2, BLK 3), `RSIZE` 2, `HIBLK` 4 and `EFBLK` 8 (high word first),
`FFBYTE` 12, `BKTSIZE` 14, `VFCSIZE` 15, `MAXREC` 16, `DEFEXT` 18, `GBC`
20, and `VERSIONS` 30, a directory's default version limit.

## The functions

`$QIO` p1 is the FIB, p2 a name, p3 the address of a word for the result
string's length, p4 the result string, p5 the attribute list. The IOSB's
first word is the status; its second longword is 0 but for an extend or
a truncate.

- `IO$_ACCESS` without `IO$M_ACCESS` looks the file up and reads
  attributes, which takes read access, or else control access. With
  `IO$M_ACCESS` it opens the file on the channel: `SS$_FILALRACC` if one
  is open there already.
- `IO$_MODIFY` checks a `FIB$W_VERLIMIT` above 32767 (`SS$_BADPARAM`),
  looks the file up, refuses a write-locked volume (`SS$_WRITLCK`), then
  writes the attributes: the first that needs control access checks it
  once, else `SS$_NOPRIV`. A file not open on the channel is marked
  revised, unless `FIB$V_NORECORD`. In V4.3 the characteristics and the
  record attributes need no access at all, and a file open on another
  channel is written anyway; `SET FILE` avoids both by opening the file
  for itself alone first (`IO$_ACCESS!IO$M_ACCESS` with `WRITE`,
  `NOREAD`, `NOWRITE`), and gets `SS$_ACCONFLICT` if another has it open.
- `IO$_CREATE` and `IO$_DELETE` make and remove files and directory
  entries; `IO$_ACPCONTROL` does volume-wide work such as disk quotas.

## Version limits

Each directory record, a name with its versions, holds the name's limit
in `DIR$W_VERLIMIT`. A directory's own record attributes hold the
default for new names in it, `FAT$W_VERSIONS`. When the XQP enters a new
name, its limit is `FIB$W_VERLIMIT`, else the directory's default, else
32767; a new version of a name keeps the name's. When the name then has
more versions than its limit, the lowest is taken out of the directory
and deleted, and the status is `SS$_FILEPURGED`, "oldest file version
purged"; one per create. A new version that would itself be the lowest
is `SS$_TOOMANYVER`, "too many higher file versions".

`SET FILE/VERSION_LIMIT=n` sends `IO$_MODIFY` with the file ID, its
directory's ID, `FIB$M_FINDFID` and `FIB$W_VERLIMIT` = n, and no
attributes. A limit of 0 in the FIB means "leave it", so SET sends 32767
for 0. n defaults to 32767; above it is `%SET-E-VALERR`. V4.3 stores the
limit only when the entry found is the name's highest version; VSI's
help says it applies to every version. Unverified which.

`SET DIRECTORY/VERSION_LIMIT=n` reads the `.DIR` file's record
attributes, sets `VERSIONS` and writes them back with `IO$_MODIFY`. It
changes only names made afterwards. A file that isn't a directory is
`%SET-E-WRITEERR` then `-SET-E-NOTDIR, file is not a directory`.

## SET VOLUME

`SET VOLUME` opens `dev:[0,0]INDEXF.SYS` for block I/O, finds the first
valid home block between VBN 2 and 100, and for every copy up to three
clusters rewrites the field, both checksums (`HM2$W_CHECKSUM1` at 58,
the sum of words 0-28, and `HM2$W_CHECKSUM2` at 510), then changes the
mounted volume's VCB in kernel mode. The fields: `/LABEL` 472 (12
characters, padded with spaces; `DISK$label` stays), `/OWNER_UIC` 44,
`/PROTECTION` 52, `/FILE_PROTECTION` 54, `/[NO]ERASE_ON_DELETE` and
`/[NO]HIGHWATER_MARKING` in `VOLCHAR` 42, `/WINDOWS` 68, `/ACCESSED` 69
(OPER), `/EXTENSION` 70, `/RETENTION` 72 and 80, `/USER_NAME` 484. A
protection only replaces the categories named. A volume not its owner's
takes control access.

## Messages

`%SET-I-MODIFIED, !AS modified` for `/LOG`; `%SET-E-WRITEERR, error
modifying !AS`, `%SET-E-READERR, error reading !AS`, `%SET-E-NOTDIR`,
`%SET-E-NOTMOD, !AS not modified`, `%SET-E-HBREAD` and `HBWRITE`, `error
reading (writing) home block of !AS`, `%SET-E-NOHOME, volume does not
contain a valid home block`, and the shared `%SET-E-VALERR, specified
value is out of legal range`. The system's: `SS$_BADATTRIB` 52 "bad
attribute control list", `SS$_ACCONFLICT` 2048 "file access conflict",
`SS$_FILALRACC` 164 "file already accessed on channel".

## SET FILE's qualifiers

`/ATTRIBUTE=(kw[,...])` (VSI's; V4.3 has none): `BKS:n`, `DEQ:n`, `EBK:n`,
`FFB:n`, `FSZ:n`, `GBC:n`, `HBK:n`, `LRL:n`, `MRS:n`, `VRS:n`,
`ORG:{IDX|REL|SEQ}`, `RAT:{BLK|CR|FTN|MSB|NONE|PRN}`,
`RFM:{FIX|STM|STMCR|STMLF|UDF|VAR|VFC}`, and dates as `CREDATE=time` and
the like. `/[NO]EXPIRATION_DATE[=time]` writes `ATR$C_EXPDATE`, 0 for
none, and takes read, write and control access. `/VERSION_LIMIT[=n]` as
above. Unverified: whether a modern XQP lets `HBK` change the
allocation, which V4.3's puts back.
