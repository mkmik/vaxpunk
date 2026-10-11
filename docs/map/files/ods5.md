+++
title = "Files-11 ODS-5"
summary = "ODS-2 extended: long, mixed-case names, deep directories, hard and soft links"
aka = ["ODS-5", "structure level 5", "extended file specifications", "EFS", "PARSE_STYLE=EXTENDED", "CASE_LOOKUP", "hard links", "symlinks", "^ escapes"]
kind = "core"
intent = "goal"
research = "outline"
refs = ["docs/adr/0007-system-disk-files-11-and-rms.md", "docs/prd/0007-wasmrun.md", "crosstools/ods/docs/names.md", "crosstools/ods/docs/file-header.md", "crosstools/ods/docs/vms-check.md", "https://wasd.vsm.com.au/htbin/helpgate/HELP/EXT_FILE_SPECS/OVERVIEW/FEATURES/ODS-5", "https://de.openvms.org/TUD2010/File_System_Changes.pdf"]

[[feature]]
id = "read"
title = "Mount an ODS-5 volume; read its ident areas and ISO Latin-1 and UCS-2 names"
status = "todo"
note = "PRD-0007 M6 tests on ODS-5 volumes; ADR-0007 chose ODS-2 alone for now"

[[feature]]
id = "write"
title = "Create, enter and delete files with long, case-preserved names"
status = "todo"
needs = ["#read"]
note = "PRD-0007 M6"

[[feature]]
id = "init"
title = "INITIALIZE/STRUCTURE=5, and SET VOLUME/STRUCTURE_LEVEL=5 to convert in place"
status = "todo"
needs = ["#write", "/files/volumes"]

[[feature]]
id = "deep-dirs"
title = "Directories deeper than ODS-2's eight levels"
status = "todo"
needs = ["#read"]

[[feature]]
id = "hard-links"
title = "Hard links: several directory entries for one header, with a link count"
status = "todo"
needs = ["#write"]

[[feature]]
id = "symlinks"
title = "Symbolic links, which RMS follows, as V8.3 and later have"
status = "todo"
needs = ["#write"]
note = "PRD-0007's path_symlink and path_readlink use them where there are any"

[[feature]]
id = "access-dates"
title = "POSIX access and attribute-change dates"
status = "todo"
needs = ["#write"]

[[needs]]
on = "/files/ods2"
why = "The same structure but for the ident area and directory name types"

[[needs]]
on = "/host/ods"
why = "ods reads and writes ODS-5 already, checked by VMS 8.4: the model and the tests"
soft = true
+++

ODS-5, structure level 5, came with OpenVMS Alpha V7.2 for files from
UNIX and Windows systems, served by Advanced Server and NFS. It is ODS-2
with a different ident area in the header and name types in directory
records: a name and type together up to 236 bytes of ISO Latin-1, or
UCS-2, with spaces, extra dots and lowercase, written in specifications
with `^` escapes; case kept as first written and ignored in lookups
unless `SET PROCESS/CASE_LOOKUP=SENSITIVE`; directories deeper than eight
levels; and specifications up to 4,095 bytes, which RMS passes in the
NAML block (`t8y:/rms/core`). `SET PROCESS/PARSE_STYLE=EXTENDED` turns the
new parsing on. Hard links, POSIX access dates and, from V8.3, symbolic
links are volume characteristics on top. `SET VOLUME/STRUCTURE_LEVEL=5`
converts an ODS-2 volume in place.

vaxpunk's executive reads and writes ODS-2 only: ADR-0007 found nothing
that needed long or mixed-case names
([ADR-0007](../../adr/0007-system-disk-files-11-and-rms.md)). The
host's `ods` reads and writes ODS-5 volumes that OpenVMS Alpha V8.4
checks clean, so the layouts are known. WASMRUN's file milestone wants
both ([PRD-0007](../../prd/0007-wasmrun.md) M6 passes on ODS-5 and
ODS-2 volumes, and recommends ODS-5), and the POSIX, GNV, NFS and CIFS
pieces of the map need it for UNIX and Windows names.
