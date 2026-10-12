+++
title = "CMS"
summary = "DECset's Code Management System: libraries of elements and generations"
aka = ["CMS", "Code Management System", "DECset", "VAX DEC/CMS", "CMS library", "RESERVE", "REPLACE", "FETCH", "00CMS"]
kind = "layered"
intent = "aspiration"
research = "outline"
status = "todo"
refs = ["https://docs.vmssoftware.com/vsi-decset-for-openvms-guide-to-the-code-management-system/", "https://docs.vmssoftware.com/vsi-decset-for-openvms-code-management-system-reference-manual/", "https://docs.vmssoftware.com/vsi-decset-for-openvms-code-management-system-callable-routines-reference-manual/"]

[[needs]]
on = "/rms/core"
why = "A library is a directory of specially formatted files"

[[needs]]
on = "/rms/sequential"
why = "Elements are text files stored as deltas"

[[needs]]
on = "/exec/lock-manager"
why = "Several users read a library at once; writes are serialized by locks"

[[needs]]
on = "/exec/uic-protection"
why = "Library access starts with VMS file protection"

[[needs]]
on = "/exec/shareable-images"
why = "The callable CMS$ routines"
soft = true

[[needs]]
on = "/gui/motif"
why = "Its DECwindows interface"
soft = true
+++

CMS, the Code Management System, is DECset's source control. A CMS
library is a VMS directory that CMS owns. Each element is a file and all
its generations, stored as the first text plus the changed lines of each
later one. `RESERVE` takes a generation out to work on and `REPLACE`
puts it back as a new generation; `FETCH` reads one without reserving
it. Variants branch a line of generations (`2A1`), classes name a set of
generations such as a release, groups name sets of elements, and every
change is in the library's history. Concurrent reservations and merging
are allowed; the VMS lock manager keeps writers apart. It has DCL
commands, a callable interface, a DECwindows interface, and works with
MMS and LSE. VSI ships it in DECset today.

On vaxpunk it is an aspiration; vaxpunk itself lives in git.
