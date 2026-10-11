+++
title = "DBMS"
summary = "DEC's CODASYL network database, now Oracle CODASYL DBMS"
aka = ["VAX DBMS", "Oracle CODASYL DBMS", "CODASYL", "DBO", "DML", "FDML", "DBQ", "subschema"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://oracle.com/technetwork/database/database-technologies/rdb/dbms-131998.pdf", "https://docs.vmssoftware.com/vsi-cobol-for-openvms-dbms-database-programming-manual/"]

[[needs]]
on = "//exec/lock-manager"
why = "Page and record locks between users"

[[needs]]
on = "//exec/global-sections"
why = "Buffers shared by the processes attached to a database"
soft = true

[[needs]]
on = "//exec/decdtm"
why = "Two-phase commit across databases"
soft = true

[[needs]]
on = "//rms/core"
why = "Root files, storage areas, journals"

[[needs]]
on = "//lang/cobol"
why = "COBOL's DML is the main way programs use it"
soft = true

[[needs]]
on = "//layered/cdd"
why = "Subschemas can be taken from the dictionary"
soft = true

[[needs]]
fuzzy = "Its storage engine, shared with Rdb (KODA), and the schema, subschema and storage schema DDLs"
why = "The database itself"
+++

VAX DBMS is DEC's CODASYL database: records joined in sets, so a program
walks from an owner to its members, as the network model of the 1970s
had it. Schemas, subschemas, storage schemas and security schemas are
written in their own DDLs; COBOL programs use its data manipulation
statements directly, Fortran through a preprocessor, and other
languages through a callable interface; `DBO` is the administration
utility and `DBQ` the interactive query. It shares its database kernel
with Rdb, and it went to Oracle with Rdb in 1994, which still sells it as
Oracle CODASYL DBMS.

On vaxpunk it is an aspiration. Nothing on the map needs it.
