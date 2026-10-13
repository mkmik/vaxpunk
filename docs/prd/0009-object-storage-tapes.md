# PRD-0009 — Tapes on cloud object storage

Oct 13, 2026 · @Marko Mikulicic

An aspiration, not a milestone: this sketches what tapes on object storage
would look like to an operator and how the pieces would fit, so that the
work can be picked up later without re-deciding the shape. The details
belong in the ADRs and the design document the work order asks for.

## Context and goal

On VMS a tape was where data went to leave the machine: `BACKUP` save sets
for the nightly backup and the off-site copy, software kits, files moved
between sites. An operator loaded a cartridge, `MOUNT`ed it, and `BACKUP`
or `COPY` wrote to it; the cartridge went on a shelf or into a robot's
library. Today the place data goes to leave the machine is a bucket in
object storage: S3, Google Cloud Storage, Cloudflare R2.

vaxpunk has no tape ([t8y:/drivers/tape](../map/drivers/tape.md)), and
QEMU's virt machine has nothing to drive. Its `BACKUP` writes save sets to
disk files only ([t8y:/utils/backup](../map/utils/backup.md)).

The goal is **a tape drive whose cartridges are in a bucket**. To the
operator and to programs it is a tape drive, `MOA0:`, with VMS's tape
functions, ANSI labels, `MOUNT`, `INITIALIZE`, `BACKUP` and `DIRECTORY`.
Behind it, each volume is a set of objects in a bucket the system manager
configured once, with credentials stored once.

What a day looks like, with VMS's commands as VMS takes them (BACKUP
writes its own labels and wants its tape mounted `/FOREIGN`):

```
$ MOUNT/FOREIGN MOA0: WK4101
$ BACKUP/LOG [USERS...]*.*;* MOA0:USERS.BCK/SAVE_SET/REWIND/LABEL=WK4101
$ DISMOUNT MOA0:

$ MOUNT MOA0: WK4101
%MOUNT-I-MOUNTED, WK4101 mounted on _MOA0:
$ DIRECTORY MOA0:
USERS.BCK;1
$ DISMOUNT MOA0:

$ MOUNT/FOREIGN MOA0: WK4101
$ BACKUP/LIST MOA0:USERS.BCK/SAVE_SET
$ BACKUP MOA0:USERS.BCK/SAVE_SET/SELECT=[SMITH...] DKB0:[SMITH...]
```

Nothing in it mentions a cloud. The system manager said where the drive's
library is, once, in the startup procedure, and gave the keys, once, to a
utility that keeps them out of reach.

Goals:

- **A tape drive, `MOA0:`**, with the `$QIO` functions VMS's BACKUP uses
  on a tape: `IO$_READLBLK`, `IO$_WRITELBLK`, `IO$_WRITEOF`,
  `IO$_REWIND`, `IO$_REWINDOFF`, `IO$_UNLOAD`, `IO$_SKIPFILE` and
  `IO$_SKIPRECORD` both ways, `IO$_SENSEMODE`, `IO$_SETMODE`,
  `IO$_PACKACK`; tape marks and the end of the tape reported as
  `SS$_ENDOFFILE` and `SS$_ENDOFTAPE`. Blocks up to 65,535 bytes.
- **The drive's library is a bucket** on S3, GCS or R2, or any server
  that speaks S3's protocol.
- **A volume is a label in the library.** `MOUNT MOA0: WK4101` loads
  `WK4101` as a robot loads a cartridge.
- **`BACKUP` to and from it**, appending save sets to a tape as on real
  tape, and save sets spanning volumes.
- **Files on it are files.** `DIRECTORY`, `TYPE`, `COPY` and RMS read and
  write an ANSI-labelled volume, as on VMS.
- **Credentials kept once, by the system manager**, in a file only the
  system can read, never on a command line, never per mount.
- **A volume can leave the system.** Its objects, concatenated in order,
  are a SIMH `.tap` tape image, which an emulated VAX mounts, and a `.tap`
  image cut into objects is a volume.

## Non-goals

- Azure Blob Storage, which doesn't speak S3's protocol, and each
  provider's own API. S3's protocol reaches all three providers we want
  (*Stores and their credentials*).
- Cloud identity beyond access keys: IAM roles, instance metadata,
  OAuth, STS. vaxpunk runs in QEMU on someone's machine; it has a key.
- Archive storage classes that must be restored before a read (S3
  Glacier, GCS Archive): a mount would wait hours. A bucket's lifecycle
  rule may move old volumes there; getting them back is the provider's
  console's business.
- Reading arbitrary objects as files, or a bucket as a disk. A volume holds
  what a tape holds; `COPY` to and from a store (*Work order* step 1) is
  the way to reach a single object.
- Tape robots' own command sets, ABS and MDMS
  ([t8y:/layered/abs](../map/layered/abs.md)). `MOUNT` by label is all the
  robot there is.
- Encryption at rest beyond the provider's. `BACKUP/ENCRYPT` is BACKUP's
  own feature.
- Physical tape drives, SCSI tapes through virtio-scsi.

## What object storage can and can't do

A tape is written from the front, block after block, with tape marks
between files, and read back the same way. Writing in the middle throws
away everything after. Appending means positioning after the last file
and writing on.

Ordinary objects can't be appended to: an object is written whole, by one
`PUT` or one multipart upload, and replaced whole. Three exceptions exist,
none common to the providers we want:

| Store | Append | Where |
| --- | --- | --- |
| Amazon S3 | `PutObject` with `x-amz-write-offset-bytes` equal to the object's size | Only directory buckets in S3 Express One Zone |
| Google Cloud Storage | Appendable objects, over gRPC | Only zonal buckets (Rapid storage), which need hierarchical namespace |
| Azure | Append blobs | Not S3's protocol (a non-goal) |
| Cloudflare R2, S3 Standard, GCS regional | None | — |

Multipart upload looks like append but isn't: nothing is visible until the
upload completes, so a system that stops halfway through a 2 GB save set
leaves nothing at all where a real tape would have 1 GB of readable
blocks; and parts are at least 5 MiB.

So the design never appends to an object. **A volume is many small
objects, each written once**: a tape's blocks are cut into chunks, named
in order. Writing on means adding the next chunk; whatever was written
before a crash stays readable, as on tape. What object storage does give,
and the design leans on:

- **Strong read-after-write consistency** (S3 since December 2020, GCS
  and R2 always): a chunk just written is there for the next `HEAD`.
- **Conditional writes**, create only if absent: `If-None-Match: *` on S3
  (since August 2024) and R2, `x-goog-if-generation-match: 0` on GCS.
  They make loading a cartridge an atomic claim, so two systems can't
  have one volume loaded, as one cartridge can't be in two drives.
- **SigV4 signs a hash of the body**, and the server checks it: every
  chunk is checked end to end for free.

### The volume's objects

In bucket `acme-vms-tapes`, volume `WK4101`:

```
WK4101/DRIVE                     the system that has it loaded
WK4101/VOLUME                    its current generation: 000003
WK4101/000003/000000/000000      VOL1, HDR1, HDR2           section 0
WK4101/000003/000000/TM          tape mark
WK4101/000003/000001/000000      first chunk of USERS.BCK   section 1
WK4101/000003/000001/000001      ...
WK4101/000003/000001/TM
WK4101/000003/000002/000000      EOF1, EOF2
WK4101/000003/000002/TM
WK4101/000003/000003/TM          a second tape mark: the end of the recorded tape
```

A **section** is the run of blocks between two tape marks. Each chunk
holds whole blocks in SIMH's `.tap` framing: each block preceded and
followed by its length in four bytes. A `TM` object is four zero bytes,
SIMH's tape mark. Digits sort before letters, so concatenating a
generation's objects in name order gives a SIMH tape image.

`IO$_SKIPFILE` is a `HEAD` on the next section's `TM`; the end of the
tape is a 404. No operation needs `LIST`, and none parses XML except an
error reply, so the client is `PUT`, `GET`, `HEAD` and `DELETE`.

**Chunks are 128 KB**, or one block if a block is bigger: two of BACKUP's
largest tape blocks (127 × 512 bytes) or sixteen of its default 8,192.
Readers don't depend on the size, since the framing says where each block
ends, so it can grow later with no change to the format. At 128 KB a
gigabyte is 8,192 `PUT`s, about $0.04 on S3; a request every 128 KB at a
50 ms round trip is about 2.5 MB/s, already more than a DLT drive. It
needs one HTTP/1.1 connection kept open across requests; a TLS handshake
per chunk would cost more than the chunk.

**Writing in the middle.** A tape forgets everything after the place it
writes. Two cases cover it:

- **Writing at the beginning of the tape**, which is what `INITIALIZE`
  and `BACKUP/REWIND` do: the drive starts a new generation by writing
  `VOLUME`, and writes under the new number. The old generation is no
  longer reachable at once, as on a tape; `OBJACP` deletes its objects in
  the background, and finds leftovers from a crash on the next mount by
  looking for the generations below the current one.
- **Writing anywhere else**, which in practice is BACKUP appending a save
  set: it skips to the double tape mark at the end, backs over the second
  and writes. The drive deletes the objects after the write position,
  from the last one backwards, before it writes, so that a crash leaves a
  shorter tape and never old blocks after new ones. For an append that
  is one `TM` object.

## Operator interface

### Stores and their credentials

DECnet had the same problem: a node's outbound user name and password,
which the system uses on the user's behalf, kept by `NCP DEFINE NODE ...
ACCESS` in `SYS$SYSTEM:NETNODE.DAT` under file protection, and shown
only as "set", never read back (`NML$SHOPWSET` in VAX/VMS V4.3's NML).
The design copies that. A **store** is a named remote endpoint with
credentials, kept in `SYS$SYSTEM:OBJSTORE.DAT`, protected
`(S:RWED,O:RWED,G,W)` like `SYSUAF.DAT`. A utility maintains it and
prompts for the secret without echo, as `AUTHORIZE` prompts for a
password:

```
$ RUN SYS$SYSTEM:OBJMGR
OBJMGR> ADD R2 /PROVIDER=R2 /ACCOUNT=8f2c... /ACCESS_KEY_ID=4f1a0c...
Secret access key:
%OBJMGR-I-ADDED, store R2 added
OBJMGR> ADD GCS /PROVIDER=GCS /ACCESS_KEY_ID=GOOG1E...
Secret access key:
OBJMGR> ADD LAB /PROVIDER=S3 /ENDPOINT="https://s3.lab.example:7070" -
_OBJMGR> /REGION=us-east-1 /ACCESS_KEY_ID=lab
Secret access key:
OBJMGR> SHOW *
Store  Provider  Endpoint                                   Key ID
R2     R2        https://8f2c....r2.cloudflarestorage.com   4f1a0c...
GCS    GCS       https://storage.googleapis.com             GOOG1E...
LAB    S3        https://s3.lab.example:7070                lab
```

`/PROVIDER` fills in the endpoint and region and picks the one header
that differs, GCS's create-only precondition; `/ENDPOINT` and `/REGION`
override them. One protocol covers the rest: S3 speaks it natively, R2
with region `auto`, GCS at `storage.googleapis.com` with HMAC keys from
its interoperability settings. Each request is signed with SigV4:
SHA-256 and HMAC, which Mbed TLS already provides
([ADR-0032](../adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md)).

`OBJMGR` needs `SYSPRV`. Secrets never come back out of it, and are kept
under file protection only, as DECnet kept its passwords. Encrypting them
would need a key kept somewhere the system disk isn't, and vaxpunk has no
such place: the disk image sits on the host next to everything else. The
protection that matters is on the provider's side, and `OBJMGR`'s help
says so: give each store a key that can reach only its bucket (an R2 API
token for one bucket, an HMAC key of a service account with rights on
one bucket, an IAM policy naming one bucket).

### Drives, libraries and who may use them

A store and a bucket together are a **library**. The startup procedure
binds a drive to one, with VMS's remote file syntax, as `COPY/HTTP` takes
`URL::"..."`:

```
$ SET DEVICE MOA0: /LIBRARY=R2::"acme-vms-tapes"
$ SET DEVICE MOA1: /LIBRARY=GCS::"acme-offsite"
$ SHOW DEVICE MOA0:/FULL
```

Binding needs `OPER`, as other `SET DEVICE` changes do. Users never name
a store: they use a drive, and a drive is a device like any other, with
an owner and a protection mask (`SET SECURITY/CLASS=DEVICE`), which the
system manager sets to say who may mount it. So a store needs no
protection of its own. The one place a user could name a store is `COPY`
(step 1), which reads `OBJSTORE.DAT` itself, so only those who may read
the file, holders of `SYSPRV`, can.

**Why the bucket is the library and not the volume.** An ANSI volume
label is six characters, `A-Z 0-9` and a few others; bucket names are 3
to 63 lowercase characters, and on S3 and GCS global across every
customer. A bucket also takes a console, a permission and a lifecycle
rule to make. A volume is made at the prompt. So a bucket holds many
volumes, as a robot's library holds many cartridges, and the label, which
on a real library is the barcode on the cartridge, is the prefix.

### Volumes

| Command | What happens |
| --- | --- |
| `MOUNT/FOREIGN MOA0: WK4101` | Loads `WK4101`: claims `WK4101/DRIVE` with a create-only `PUT`, reads `VOLUME`. A label that doesn't exist yet loads a blank tape, which its first write creates. VMS ignores the label with `/FOREIGN`; here it says which cartridge to load |
| `MOUNT MOA0: WK4101` | The same, then reads `VOL1` and checks the label, as on VMS, through MTAACP |
| `INITIALIZE MOA0: WK4101` | Loads it, writes a new generation with `VOL1` and nothing else, unloads |
| `DISMOUNT MOA0:` | Writes the chunk being filled, deletes `WK4101/DRIVE` |
| `DISMOUNT/NOUNLOAD MOA0:` | Writes the chunk, keeps the claim |

If another system holds the volume, `MOUNT` fails and names it, from the
`DRIVE` object. A system that crashed with a volume loaded takes it back
when it mounts it again, since `DRIVE` holds its own node name; another
system needs the operator to break the claim with `OBJMGR UNLOAD`, as an
operator would walk over to a dead system's drive.

**Several volumes.** When a save set fills a volume (a drive may be given
a capacity, `SET DEVICE MOA0:/CAPACITY=`, since a bucket never fills),
BACKUP unloads it and prompts `mount volume 2 on _MOA0: for writing`.
The drive acts as a stacker loader did: on an unload during a mount it
loads the next cartridge itself, named by BACKUP's own rule for
continuation labels (`TAPEUTIL.B32`, `MAKE_VOL1`): the first four
characters of the first label and the volume number in two digits,
`WK4101`, `WK4102`, `WK4103`. The operator answers `YES`, and BACKUP
finds the label it expects. The same rule loads the right volumes when
reading the set back, including sets written on real tapes and imported
as SIMH images.

## How it fits together

```
 BACKUP, MOUNT/FOREIGN ─────────────┐
 COPY, DIRECTORY, RMS ── MTAACP ────┤ $QIO
                                    ▼
                             MODRIVER (MOA0:)
                                    │ IRP
                                    ▼
                      OBJACP, a detached process:
                      position, the chunk being filled,
                      S3 client, SigV4, the credentials
                                    │ SSL3 over BGA0:
                                    ▼
                             S3 / GCS / R2
```

- **`MODRIVER`**, in the executive, MACRO-32, is a tape driver whose
  hardware is a process. Its start I/O routine hands each IRP to
  `OBJACP` and completes it when `OBJACP` answers, as VMS's network ACPs
  served devices whose work needed a process. It sets `DEV$V_SQD` and
  `DC$_TAPE`, which is all BACKUP looks at to treat a device as a tape.
- **`OBJACP`**, a detached process started by the startup procedure,
  holds the tape position, the chunk being filled, the S3 client and the
  credentials. TLS, sockets and SigV4 live here, in user mode, where the
  C run-time library and SSL3 already work; nothing in the executive
  speaks HTTP. It reads one chunk ahead.
- **`MTAACP`** gives labelled volumes to RMS, as on VMS
  ([t8y:/files/ansi-tape](../map/files/ansi-tape.md)). BACKUP doesn't
  need it, but `DIRECTORY` and `TYPE` of a file on the tape do.

## Testing strategy

- **A fake S3 server in the boot test**, Rust, beside `network.rs`'s web
  and TLS servers and reached the same way, through QEMU's guestfwd:
  `PUT` with both create-only headers, `GET`, `HEAD`, `DELETE`, objects
  in a map. It checks every SigV4 signature and body hash, so a wrong
  signature fails the test instead of passing a fake that ignores it.
  Four verbs and no `LIST` keep it to a few hundred lines with no new
  dependency.
- **SIMH's VAX as the oracle.** SIMH (`vax`, in Homebrew) emulates a
  TK50 on a `.tap` file; VAX/VMS 5.5-2, whose CD the repository's
  fixtures already fetch, runs on it. A save set BACKUP writes there is
  cut into chunks and restored on vaxpunk, and a vaxpunk volume
  concatenated into a `.tap` is listed and restored there. AXPbox can't
  serve: it emulates SCSI disks and CD-ROMs, not tapes (`Disk.cpp`
  answers INQUIRY with device type 0 or 5 only).
- **One real S3 implementation**, run by hand or in a nightly job:
  `versitygw` with its POSIX backend, a single Apache-2.0 Go binary.
  MinIO, the usual choice, was archived in 2026.
- **The real providers by hand**: R2, GCS through HMAC keys, S3, each with
  a bucket of the developer's.

## Decisions

The questions the first sketch left open, answered:

| Question | Answer |
| --- | --- |
| The device name | `MOA0:`. `MO` is no driver's prefix in VAX/VMS V4.3's AUTOCONFIGURE table or in OpenVMS Alpha 8.4-2L1's `SYS$LDR` (`MOU0:`, the USB mouse, is another name). BACKUP and MOUNT go by the device's characteristics, not its name |
| Writing in the middle | A new generation for writes at the beginning of the tape, deletion of what follows for the rest (*The volume's objects*) |
| The size of a chunk | 128 KB, free to grow: readers don't depend on it |
| Credentials at rest | File protection, as DECnet's; keys scoped to one bucket at the provider |
| Who may use a store | No one directly: drives have device protection, and only `SYSPRV` reads `OBJSTORE.DAT` |
| A disk cache in `OBJACP` | No: BACKUP reads and writes in order, and one chunk read ahead covers it |
| Is MTAACP worth it | Yes, but last: BACKUP doesn't need it, and `TYPE` of a file on tape does |
| An oracle | SIMH's VAX with VAX/VMS 5.5-2; AXPbox has no tape |
| GCS's conditional write | GCS has its own header, `x-goog-if-generation-match: 0`, so a store says its provider |

## Open questions

- **BACKUP's `/LABEL` lists** name continuation volumes explicitly; the
  stacker rule doesn't follow them. An operator command to load a named
  cartridge would, at the cost of a second terminal.
- **versitygw's conditional writes**: whether its POSIX backend honours
  `If-None-Match: *`, checked in step 1.

## Work order

Each step ends with something to run.

1. **`COPY` to and from a store.** `OBJMGR` and `OBJSTORE.DAT`, the S3
   client with SigV4 over a kept-open connection, and
   `COPY X.BCK R2::"acme-vms-tapes/x.bck"` and back, next to `COPY/HTTP`.
   Already worth having: `BACKUP` to a disk save set, then `COPY` it out.
   The fake S3 server lands here.
2. **`MODRIVER` and `OBJACP`**, `MOUNT/FOREIGN` with a label, the tape
   `$QIO` functions, generations. A test program writes blocks and tape
   marks, rewinds, skips both ways and reads them back; the fake's
   objects concatenate to a valid `.tap`.
3. **`BACKUP` to tape**: `/REWIND`, appending, `BACKUP/LIST` from tape;
   a save set written by VAX/VMS under SIMH restored, and one of ours
   restored there.
4. **`INITIALIZE`, the `DRIVE` claim** and its errors, `OBJMGR UNLOAD`.
5. **MTAACP**: `MOUNT` with labels, `DIRECTORY`, `TYPE`, `COPY` of tape
   files.
6. **Several volumes**: `/CAPACITY`, the stacker, a three-volume set
   from SIMH restored.
