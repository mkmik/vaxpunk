+++
title = "DATATRIEVE"
summary = "Query and report writer over RMS files, Rdb and DBMS"
aka = ["DATATRIEVE", "DTR", "VAX DATATRIEVE", "DTR32", "wombat"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://en.wikipedia.org/wiki/DATATRIEVE", "https://docs.vmssoftware.com/vsi-datatrieve-user-guide/", "https://docs.vmssoftware.com/vsi-datatrieve-reference-manual/"]

[[needs]]
on = "/rms/indexed"
why = "Domains are mostly RMS files, read by key"

[[needs]]
on = "/rms/sequential"
why = "Flat files are domains too"

[[needs]]
on = "/layered/cdd"
why = "Record and domain definitions live in the dictionary"

[[needs]]
on = "/drivers/terminal"
why = "Interactive queries and reports at a terminal"

[[needs]]
on = "/layered/rdb"
why = "Relational databases as domains"
soft = true

[[needs]]
on = "/layered/dbms"
why = "CODASYL databases as domains"
soft = true

[[needs]]
on = "/layered/fms"
why = "Forms for STORE and MODIFY"
soft = true
+++

DATATRIEVE is DEC's query language and report writer, from the PDP-11 in
1977 and the VAX in 1981, one of the first fourth-generation languages.
A user declares a domain, a file or database with a record definition
from CDD, and then asks for data in near-English commands: `FIND`,
`SELECT`, `PRINT`, `MODIFY`, `STORE`, `SORT`, joins of domains into
views, and a report writer. It reads sequential and indexed RMS files,
Rdb and DBMS, and has a callable interface. Its help answers `HELP
WOMBAT`. VSI maintains it today.

On vaxpunk it is an aspiration. Over RMS indexed files it would be a
natural way to query the home-office inventory's data, but nothing
needs it.
