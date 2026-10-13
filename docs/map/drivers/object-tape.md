+++
title = "Object storage tapes"
summary = "MOA0:, a tape drive whose cartridges are objects in an S3, GCS or R2 bucket"
aka = ["MOA0:", "MODRIVER", "OBJACP", "OBJMGR", "OBJSTORE.DAT", "S3", "GCS", "R2", "virtual tape", "cloud tape", "SIMH .tap"]
kind = "glue"
intent = "aspiration"
research = "outline"
refs = ["docs/prd/0009-object-storage-tapes.md"]

[[feature]]
id = "copy-store"
title = "OBJMGR and OBJSTORE.DAT, a SigV4 S3 client, COPY to and from store::\"bucket/key\""
status = "todo"
note = "PRD-0009 step 1, with the fake S3 server in the boot test"

[[feature]]
id = "foreign"
title = "MODRIVER and OBJACP: BACKUP's tape $QIO functions, MOUNT/FOREIGN by label, generations, chunks that concatenate to a SIMH .tap"
status = "todo"
needs = ["#copy-store", "/exec/qio"]
note = "PRD-0009 step 2"

[[feature]]
id = "backup"
title = "BACKUP save sets on it, /REWIND or appended, BACKUP/LIST; SIMH's VAX/VMS reads them"
status = "todo"
needs = ["#foreign", "/utils/backup"]
note = "PRD-0009 step 3"

[[feature]]
id = "claims"
title = "INITIALIZE, the DRIVE claim by a create-only PUT, OBJMGR UNLOAD"
status = "todo"
needs = ["#foreign", "/utils/mount"]
note = "PRD-0009 step 4; step 5, labelled files through MTAACP, is t8y:/files/ansi-tape"

[[feature]]
id = "multi-volume"
title = "A save set across volumes: /CAPACITY, the next volume loaded by BACKUP's label rule"
status = "todo"
needs = ["#backup"]
note = "PRD-0009 step 6"

[[needs]]
on = "/net/ssl"
why = "Every request to the store is https"

[[needs]]
on = "/net/tcpip-qio"
why = "OBJACP's connection to the store"

[[needs]]
on = "/rtl/crtl"
why = "OBJACP's S3 client is C, beside SSL3"

[[needs]]
on = "/exec/processes"
why = "OBJACP is a detached process the driver hands its IRPs to"

[[needs]]
on = "/exec/uic-protection#devices"
why = "A drive's protection mask says who may mount it; stores have none of their own"
soft = true
+++

A tape drive whose library is a bucket. `MOA0:` takes the tape `$QIO`
functions VMS's BACKUP uses; `OBJACP`, a detached process, turns them
into `PUT`, `GET`, `HEAD` and `DELETE` requests, signed with SigV4, on
S3, GCS (HMAC keys) or R2. A volume is a six-character label, a prefix in
the bucket, as a barcode names a cartridge in a robot's library. Since
ordinary objects can't be appended to, a volume is cut into chunks of
128 KB, each written once; its objects concatenated in order are a SIMH
`.tap` image. Rewriting from the start makes a new generation, so
`INITIALIZE` and `BACKUP/REWIND` don't delete a tape's worth of objects
first. The system manager stores each store's keys once with `OBJMGR`,
as DECnet kept outbound access in `NETNODE.DAT`, and binds a drive to a
library with `SET DEVICE MOA0: /LIBRARY=R2::"bucket"`.

Nothing is built. [PRD-0009](../../prd/0009-object-storage-tapes.md)
sketches it.
