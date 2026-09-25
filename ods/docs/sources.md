# Sources

No code was copied from any implementation. Structure layouts come from the
documents below and were checked against real VMS disks; where a detail
came from somewhere else it says so in the note that uses it.

| Source | What it gave | Pin |
| --- | --- | --- |
| Kirby McCoy, *VMS File System Internals*, Digital Press 1990 (EY-F575E-DP). Scanned with OCR at bitsavers: `pdf/dec/vax/vms/training/EY-F575E-DP_VMS_File_System_Internals_1990.pdf` | ODS-2: every structure in chapter 2 (volume, file ID, header, ident, map, directories, reserved files, index file, home block, SCB, bitmap), INITIALIZE in section 3.2 | SHA-256 `2865ad0b32596ded83befe7dcf0e63d431cfd8dc8109575e1f17f5e985216341` |
| VSI, *OpenVMS Guide to Extended File Specifications* (docs.vmssoftware.com) | ODS-5 name rules: lengths, forbidden characters, `^` escapes, case preservation, wildcards | SHA-256 `9b2c7a469bbe07f24e7033f382aa71de191fc6f1fee6289a2bd877bb6aef3ceb` |
| FreeVMS `lib/src/fi5def.h` (github.com/rroart/freevms, GPL-2.0), transcribing DEC's `$FI5DEF` | The ODS-5 ident area offsets: no public DEC document gives them. Read for the facts only, as AGENTS.md allows for FreeVMS | commit `master`, 2026-09-25 |
| A volume VMS 7.1 INITIALIZEd, `samples/empty_ods2_volume.dsk.gz` in github.com/allenpomeroy/ods2v2 (MIT) | What INITIALIZE writes, field by field: see [initialize.md](initialize.md). Used as data, not code | commit `30068dc3`, fixture `vms-7.1-init.dsk` |
| The fixtures in `fixtures/` | Everything above, confirmed on VMS V1.0 (1978) through OpenVMS Alpha 8.4-2L1 (2016) volumes; directory order; empty directories; version limits; empty header templates; simh's disk footer | `fixtures/SHA256SUMS` |
| OpenVMS Alpha V8.4-2L1 itself, run in AXPbox ([vms-check.md](vms-check.md)) | ODS-5 as VMS writes it: the `vms-ods5.img` fixture (`vms/samples.dcl`, with DUMP/HEADER of every file in its log); and ANALYZE/DISK_STRUCTURE's verdict on volumes `ods` writes | `vms/` |

The ODS-1 specification (for RSX-11) is on bitsavers too
(`pdf/dec/pdp11/rsx11m_s/Files-11_ODS-1_Spec_Sep86.txt`); ODS-2 inherits
its retrieval pointer and checksum ideas but not its layouts, so it was not
used.
