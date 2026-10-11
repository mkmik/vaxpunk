+++
title = "CDD/Repository"
summary = "The data dictionary: record definitions shared by compilers and products"
aka = ["CDD", "VAX CDD", "CDD/Plus", "CDD/Repository", "Common Data Dictionary", "CDO", "DMU", "CDDL", "CDD$DEFAULT", "CDD$COMPATIBILITY", "Oracle CDD/Repository"]
kind = "layered"
intent = "aspiration"
research = "outline"
refs = ["https://oracle.com/a/tech/docs/collateral/cdd74100-install-guide.pdf", "https://oracle.com/a/tech/docs/collateral/cdd-release-notes7410.pdf", "https://community.hpe.com/t5/operating-system-openvms/acms-dev-without-oracle-cdd/m-p/3106295", "https://wasd.vsm.com.au/sys$common/syshlp/cc$help/5492profile_037.html", "https://en.wikipedia.org/wiki/Oracle_Rdb"]

[[feature]]
id = "records"
title = "Field and record definitions that compilers, DECforms and ACMS copy in"
status = "todo"
note = "C's #pragma dictionary, COBOL's COPY FROM DICTIONARY, IFDL's COPY, MMS's ^ references"

[[feature]]
id = "cdo"
title = "CDO, the Common Dictionary Operator: define, change and show definitions"
status = "todo"
needs = [":records"]

[[feature]]
id = "dmu"
title = "DMU-format dictionaries and CDDL, as in VAX CDD V3"
status = "todo"
needs = [":records"]

[[feature]]
id = "repository"
title = "Repositories in an Rdb database under an anchor: versions, where-used"
status = "todo"
needs = [":cdo", "//layered/rdb", "//exec/decdtm:services"]

[[needs]]
on = "//rms/core"
why = "Dictionaries live in files under an anchor directory"

[[needs]]
on = "//exec/logical-names"
why = "CDD$DEFAULT and CDD$COMPATIBILITY name the dictionaries"

[[needs]]
fuzzy = "Where definitions are stored: an Rdb database, as CDD/Repository does, or files of its own, as the DMU dictionary did"
why = "The home-office inventory idea keeps Rdb out"
+++

The Common Data Dictionary keeps the definitions of records and their
fields in one place, so every program and product uses the same layout.
COBOL, C, Fortran, Pascal and other compilers copy a record definition
in at compile time; DATATRIEVE, ACMS, TDMS, DECforms, Rdb and DBMS read
it too. VAX CDD came first, with the CDDL language and the DMU utility.
CDD/Plus added CDO, the Common Dictionary Operator, and a repository
that tracks versions and which definition uses which. Oracle bought it
with Rdb in 1994 and sells it as Oracle CDD/Repository. Its current
release needs Rdb, which holds each repository as `CDD$DATABASE.RDB`
under an anchor directory, and DECdtm for every transaction.

On vaxpunk it is an aspiration. Marko's home-office inventory idea uses
CDD for its record definitions but keeps Rdb out, while CDD/Repository
stores everything in Rdb. Which dictionary vaxpunk would want, the older
DMU kind or a CDO repository on something lighter than Rdb, is open.
