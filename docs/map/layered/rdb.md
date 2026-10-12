+++
title = "Rdb"
summary = "DEC's relational database, now Oracle Rdb: SQL and RDO on VMS"
aka = ["Rdb", "Rdb/VMS", "VAX Rdb/VMS", "Oracle Rdb", "RDO", "RMU", "SQL$", "SQL module language", "KODA"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://en.wikipedia.org/wiki/Oracle_Rdb", "https://www.oracle.com/a/otn/docs/rdb074-install-guide.pdf"]

[[needs]]
on = "/exec/lock-manager"
why = "Page and record locks between users, and across a cluster"

[[needs]]
on = "/exec/global-sections"
why = "Global buffers shared by every process attached to a database"

[[needs]]
on = "/exec/decdtm"
why = "Transactions that span databases and other resource managers"

[[needs]]
on = "/rms/core"
why = "Root files, storage areas, journals"

[[needs]]
on = "/exec/cluster"
why = "One database open on every node of a cluster"
soft = true

[[needs]]
fuzzy = "Its storage engine: storage areas, before- and after-image journals, snapshots, RMU"
why = "The database itself, KODA, shared with DBMS"

[[needs]]
fuzzy = "SQL precompilers and the SQL module language for each compiler"
why = "How programs reach it"
+++

Rdb is the relational database DEC shipped for VMS from 1984, as VAX
Rdb/VMS, part of the VAX Information Architecture with DATATRIEVE, ACMS
and CDD. Programs use RDO, its first query language, or SQL, embedded
through a precompiler, in separate SQL modules, or dynamically; `RMU`
manages, backs up and verifies databases. It runs on a database kernel,
KODA, that does the locking, journaling and buffering, and it uses the
VMS lock manager and clusters to share one database among many
processes and nodes. DEC sold it to Oracle in 1994; Oracle Rdb still
ships for Alpha and Itanium and is being ported to x86.

On vaxpunk it is an aspiration. Marko's home-office inventory idea
leaves it out on purpose and uses RMS indexed files. CDD/Repository
needs it to store its own definitions.
