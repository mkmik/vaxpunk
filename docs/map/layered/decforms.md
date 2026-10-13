+++
title = "DECforms"
summary = "Forms on VT terminals and Motif, described in IFDL, run by the Form Manager"
aka = ["DECforms", "IFDL", "FIMS", "Form Manager", "FDE", "FORMS TRANSLATE", "FORMS BACK_TRANSLATE", "FORMS EXTRACT", ".IFDL", ".FORM", "DECforms Web Connector"]
kind = "layered"
intent = "aspiration"
research = "outline"
refs = ["vms/sysexe/forms/README.md", "docs/adr/0034-forms-grammar-as-data-and-form-files-as-trees.md", "https://zx.net.nz/mirror/www.compaq.com/info/SP2990/SP2990PF.PDF", "https://docs.vmssoftware.com/vsi-decforms-for-openvms-ifdl-reference-manual/", "https://docs.vmssoftware.com/vsi-decforms-for-openvms-programmer-s-reference-manual/", "http://neilrieck.net/misc/pdf/vms-docs/DecForms/5616GD_COMM_UTIL.PDF", "https://h41379.www4.hpe.com/doc/721final/6609/6609pro_007.html", "https://ksi.cpsc.ucalgary.ca/archives/HTML-WG/html-wg-95q2.messages/0680.html"]

[[feature]]
id = "ifdl"
title = "IFDL and its translator: .IFDL source to binary .FORM files, and back"
status = "done"
note = "FORMS TRANSLATE and FORMS BACK_TRANSLATE, FORMS.EXE in BLISS-64; DECforms's sample and demo forms translate and come back the same"

[[feature]]
id = "copy"
title = "IFDL COPY of other files and of text library modules"
status = "todo"
needs = ["#ifdl", "/rtl/lbr"]

[[feature]]
id = "checks"
title = "The rest of the translator's checks: fields in their viewports, pictures, VALUEs, keys"
status = "todo"
needs = ["#ifdl"]
note = "FORMS checks names, declarations, references and numbers; vms/sysexe/forms/check.b64 lists what it doesn't"

[[feature]]
id = "character-cell"
title = "Panels on VT terminals: fields, literals, validation, help, function keys"
status = "todo"
needs = ["#ifdl", "/drivers/terminal"]

[[feature]]
id = "form-manager"
title = "The Form Manager: ENABLE, SEND, RECEIVE, TRANSCEIVE, DISABLE, CANCEL"
status = "todo"
needs = ["#character-cell"]

[[feature]]
id = "form-images"
title = "FORMS EXTRACT OBJECT: forms linked into an image or a shareable image"
status = "todo"
needs = ["#form-manager", "/utils/linker"]

[[feature]]
id = "escape-units"
title = "Escape units: the form calls procedures in the program or another image"
status = "todo"
needs = ["#form-manager", "/exec/image-activator"]

[[feature]]
id = "cdd-records"
title = "IFDL COPY of record definitions from CDD"
status = "todo"
needs = ["#ifdl", "/layered/cdd#records"]

[[feature]]
id = "fde"
title = "The Form Development Environment and the VT panel editor"
status = "todo"
needs = ["#character-cell"]

[[feature]]
id = "motif"
title = "Layouts for DECwindows Motif workstations and X terminals"
status = "todo"
needs = ["#form-manager", "/gui/motif"]

[[feature]]
id = "fms-converter"
title = "Converting FMS forms to IFDL"
status = "todo"
needs = ["#ifdl", "/layered/fms"]

[[feature]]
id = "web-connector"
title = "DECforms Web Connector: the same forms in a web browser"
status = "todo"
needs = ["#form-manager", "/net/web-server"]

[[needs]]
on = "/drivers/terminal"
why = "Panels are drawn and read on VT terminals"

[[needs]]
on = "/exec/calling-standard"
why = "Any language calls the Form Manager; records go by descriptor"

[[needs]]
on = "/exec/shareable-images"
why = "The Form Manager is a library programs call; forms can be images"

[[needs]]
on = "/exec/logical-names"
why = "FORMS$IMAGE and the trace logicals find images and files"

[[needs]]
on = "/rms/core"
why = "Reads .FORM files and writes trace files"

[[needs]]
on = "/layered/cdd"
why = "Record definitions shared with the program; optional"
soft = true
+++

DECforms is the forms system DEC built after FMS and TDMS, an
implementation of the CODASYL FIMS draft standard. A form is written in
IFDL, the Independent Form Description Language: panels of fields and
literals, grouped into layouts for each kind of terminal or language, a
form record that matches a record in the program, and responses that say
what happens when the user presses a key or leaves a field. The
translator turns IFDL into a binary `.FORM` file. At run time the
program calls the Form Manager with six requests: `ENABLE`, `SEND` (data
to the form), `RECEIVE` (data from it), `TRANSCEIVE` (both), `DISABLE`
and `CANCEL`. The form does the dialog, validation and help on its own,
so the program never touches the screen. Forms run on VT terminals and,
with other layouts, on DECwindows Motif. It replaced FMS and TDMS, and
ACMS uses it for its exchange steps. VSI sells it today.

On vaxpunk it is an aspiration, needed by ACMS and by Marko's
home-office inventory idea. The translator is built: `FORMS TRANSLATE`
and `FORMS BACK_TRANSLATE`, `FORMS.EXE` in BLISS-64
([vms/sysexe/forms](../../../vms/sysexe/forms/README.md)), with
DECforms's messages, and a `.FORM` file of vaxpunk's own, the parse tree
([ADR-0034](../../adr/0034-forms-grammar-as-data-and-form-files-as-trees.md)).
The Form Manager isn't, so no form runs yet. The character-cell side
needs only the terminal driver and the calling standard; the Motif side
needs the DECwindows work in [t8y:/gui](../gui/AREA.md).
