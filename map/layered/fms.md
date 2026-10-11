+++
title = "FMS"
summary = "The Form Management System: VT forms through the FDV$ Form Driver"
aka = ["FMS", "VAX FMS", "Form Management System", "FDV$", "FDVSHR", "form library", ".FLB", ".FRM", "FMS/EDIT"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://docs.vmssoftware.com/introduction-to-vsi-fms/", "https://docs.vmssoftware.com/vsi-fms-form-driver-reference-manual/", "https://uprrp2.uprrp.edu/sys$common/syshlp/HPFMSV025.RELEASE_NOTES"]

[[needs]]
on = "//drivers/terminal"
why = "The Form Driver draws forms on VT terminals and reads keys"

[[needs]]
on = "//exec/shareable-images"
why = "The Form Driver is FDVSHR.EXE, called by every FMS program"

[[needs]]
on = "//exec/calling-standard"
why = "FDV$ calls from any language"

[[needs]]
on = "//rms/core"
why = "Forms in .FRM files and .FLB form libraries"
+++

FMS, the Form Management System, is DEC's older forms product for VT
terminals, on RSX and VMS. Forms are drawn in the form editor, kept as
`.FRM` files and grouped into `.FLB` libraries; a program attaches a
terminal and shows and reads forms field by field with `FDV$` calls to
the Form Driver, `FDVSHR.EXE`. Utilities translate, test and list forms
and turn them into object modules. ALL-IN-1 built its screens on FMS.
DECforms replaced it and can convert its forms; VSI still sells it.

On vaxpunk it is an aspiration. Nothing needs it but DECforms'
converter.
