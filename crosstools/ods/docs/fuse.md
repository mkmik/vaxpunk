# FUSE mapping: proposal

The PRD leaves the POSIX-to-VMS mapping to a design session. This note
proposes an answer to each open question, as input to that session.
`ods-fuse` implements the read-only half as proposed here, so the choices
can be tried rather than argued about; everything in it that depends on
them sits in one module (`crates/ods-fuse/src/map.rs`).

## Versions

**Proposal**: a directory lists each name once, as its highest version,
under the plain name. Every version, the highest included, is also
reachable as `name;N`, but not listed unless the mount asks for it
(`-o versions`, which lists `name;N` for all versions but the highest).

Why: tools that walk trees (`ls`, `grep -r`, Finder, editors) should see
one file per name, the current one. VMS's DIRECTORY lists every version,
but on a Mac that would send every recursive search through each old
version too. Explicit paths like `cat 'LOGIN.COM;3'` still reach them.

**Writes** (read-write mount, later): creating or truncating `name` (open
with `O_CREAT` or `O_TRUNC`) makes a new version when the file is closed;
writing in place into an existing file without truncating it modifies the
current version, as VMS does for block I/O. Editors that save by writing a
temporary file and renaming it over the original produce a new version
through the rename: the rename becomes "enter as the next version", and the
old version stays. Version limits purge as on VMS.

## Names

**Proposal**: show names as stored, except that an empty type drops its
dot (`MAKEFILE.` shows as `MAKEFILE`) unless a subdirectory of the same
name exists, in which case the file keeps the dot. Directories show
without `.DIR;1`. Lookups compare case-blind (the file system does, as
ODS-5 does), so `readme.txt` finds `README.TXT` on any host, and a name
without a dot finds the subdirectory first, then the file with an empty
type.

For writes, names from the host are mapped reversibly:

- ODS-2: letters are uppercased; characters outside `A-Z 0-9 $ - _`, a
  second dot, and names over 39.39 are refused with `EINVAL`. No escaping:
  a name that needs escaping does not belong on ODS-2.
- ODS-5: names are stored as given (ISO Latin-1). Characters VMS forbids
  (`" * \ : < > / ? |` and controls) are refused; characters outside
  Latin-1 would need UCS-2 names, which `ods-core` does not write yet.
- A name without a dot gets an empty type: `Makefile` is stored as
  `MAKEFILE.` on ODS-2 and `Makefile.` on ODS-5, and lists as `MAKEFILE`
  or `Makefile`.

## Directory syntax

Paths are Unix paths; `[A.B]` never appears. `.DIR;1` files are not shown
as files: a directory is a POSIX directory. The MFD is the mount root; its
reserved files (`INDEXF.SYS` and friends) are shown, read-only, since they
are real files a VMS user sees too.

## File sizes of record files

**Proposal**: a text file (VAR or VFC with carriage control, or a stream
format) reads as LF-terminated lines, like `ods copy-out`, and its
`st_size` is the size of that text. Everything else reads as its bytes up
to the end of file. The conversion is computed once per file and kept as
checkpoints every 64 kB of output (which record starts there), so reads
seek without converting from the start and nothing holds a whole file in
memory. The raw bytes stay available through the CLI.

The alternative, both views (`name` and `name.raw`), doubles every
directory listing; a mount option could add it if needed.

## Permissions

`st_mode` comes from the protection code: owner bits from the owner
category (R→r, W→w, E→x; delete has no POSIX equivalent), group from group,
other from world. Directories get x from E. Files are owned by the user who
mounted the volume: UICs mean nothing on the host. The UIC and the full
protection code are in extended attributes.

## Timestamps

| POSIX | VMS |
| --- | --- |
| `st_mtime` | revision date, else creation date |
| `st_birthtime` (macOS) | creation date |
| `st_atime` | access date (ODS-5), else as mtime |
| `st_ctime` | attribute change date (ODS-5), else as mtime |

Times are taken as UTC.

## Extended attributes

In the `vms.` namespace, as text: `vms.fid` `(12,1,0)`, `vms.version`,
`vms.rfm` (`VAR`), `vms.rat` (`CR`), `vms.mrs`, `vms.lrl`, `vms.org`,
`vms.fch` (characteristics), `vms.uic` (`[1,4]`), `vms.prot`
(`S:RWED,O:RWED,G:RE,W:`), `vms.eof` (bytes), `vms.created`,
`vms.revised`, `vms.expires`, `vms.backup` (VMS format dates).

## macOS metadata

`com.apple.*` extended attributes are refused (`ENOTSUP`), and `._name`
AppleDouble files and `.DS_Store` do not exist: lookups fail, creation
fails. On macFUSE the mount passes `noappledouble` so Finder does not try.

## macFUSE or FUSE-T

The code uses only calls both support (the `fuser` crate over libfuse's
API). macFUSE's kernel extension gives full extended attributes; FUSE-T
goes through NFS, whose extended attribute support is weaker, which would
mainly cost the `vms.` attributes. Recommendation: macFUSE for now, try
FUSE-T when it matters that nothing needs a kernel extension.
