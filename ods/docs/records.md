# Records and text

The file system stores bytes; RMS gives them record structure, described by
the record attributes (see [file-header.md](file-header.md)). `ods-image`
reads the formats text uses (`src/records.rs`):

| Format | Layout |
| --- | --- |
| VAR | 16-bit little-endian length, data, one pad byte if the length is odd. In files with `BLK` a length of 0xFFFF means the rest of the block is unused |
| VFC | VAR whose first `VFCSIZE` bytes (default 2) are print control |
| FIX | `RSIZE` bytes each, padded to even length |
| STMLF, STMCR, STM | bytes; records end at LF, CR, or CR LF |

UDF files, and relative and indexed files, have no records a text reader
can use.

## Copy modes

Nothing is converted unless asked:

- **binary**: the bytes up to the end of file, both ways. Coming in, the
  file is UDF unless record attributes are given.
- **records-to-lines** (out): each record becomes a line ending in LF; VFC
  control bytes are dropped (print control is not applied).
- **lines-to-records** (in): each line (without LF, or CR LF) becomes a VAR
  record with CR carriage control.

`copy-out` without `--mode` uses records-to-lines for files that are text
beyond doubt (VAR or VFC with carriage control, or a stream format) and
binary for everything else, and says which it used. `copy-in` defaults to
binary.

## Dates

VMS time is a 64-bit count of 100 ns units since 17-Nov-1858 00:00;
1-Jan-1970 is 0x007C95674BEB4000. VMS kept local time; `ods` writes UTC.
