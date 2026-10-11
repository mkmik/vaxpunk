+++
title = "RMS Journaling"
summary = "After-image, before-image and recovery-unit journals for RMS files"
aka = ["RMS Journaling for OpenVMS", "AI journaling", "BI journaling", "RU journaling", "SET FILE/AI_JOURNAL", "SET FILE/BI_JOURNAL", "SET FILE/RU_JOURNAL", "RECOVER/RMS_FILE", "XABJNL", "XABRU"]
kind = "layered"
intent = "aspiration"
research = "outline"
refs = ["docs/prd/0008-rms-record-and-indexed-files.md", "https://wasd.vsm.com.au/htbin/helpgate/HELP/RECOVER", "https://wasd.vsm.com.au/htbin/helpgate/HELP/SET/FILE/_RU_JOURNAL/EXAMPLES"]

[[feature]]
id = "after-image"
title = "After-image journals, and RECOVER/RMS_FILE/FORWARD from a backup"
status = "todo"
note = "PRD-0008 non-goal"

[[feature]]
id = "before-image"
title = "Before-image journals, and RECOVER/RMS_FILE/BACKWARD"
status = "todo"
note = "PRD-0008 non-goal"

[[feature]]
id = "recovery-units"
title = "Recovery units: a transaction's changes to RMS files kept or undone together"
status = "todo"
needs = ["//exec/decdtm:resource-managers", "//rms/sharing:record-locks"]
note = "PRD-0008 non-goal: no DECdtm"

[[needs]]
on = "//rms/core"
why = "Every change a service makes is written to the journal first"

[[needs]]
on = "//rms/indexed"
why = "Most journaled files are indexed"

[[needs]]
on = "//utils/backup"
why = "Recovery forward starts from a backup copy of the file"
soft = true
+++

RMS Journaling for OpenVMS, a separately licensed option, adds three
kinds of journal to RMS files, chosen per file with `SET FILE`. An
after-image journal records each change, so a file restored from a
backup can be rolled forward to its last state with `RECOVER/RMS_FILE/FORWARD`.
A before-image journal records what each change overwrote, so a damaged
file can be rolled back. Recovery-unit journaling makes a group of
changes, to several files, atomic: they all happen or none do, within a
DECdtm transaction. Without it, RMS files are as good as their last
write; ACMS applications used it to make RMS transactional.

vaxpunk has no journaling
([PRD-0008](../../docs/prd/0008-rms-record-and-indexed-files.md) lists
it with DECdtm as a non-goal). The signature application, an ACMS
inventory system on RMS indexed files, would want recovery units.
