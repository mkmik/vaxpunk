+++
title = "MAIL"
summary = "VMS MAIL: messages between users, folders in MAIL.MAI, and mail between systems"
aka = ["MAIL", "VMS MAIL", "MAIL.EXE", "MAIL.MAI", "MAILSHR", "MAIL$", "callable mail", "VMSMAIL_PROFILE.DATA", "NEWMAIL", "WASTEBASKET", "MAIL$EDIT", "MAIL-11", "NODE::USER", "SMTP%", "MIME"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/prd/0002-networking.md", "docs/prd/0003-multi-user-vms.md", "docs/prd/0008-rms-record-and-indexed-files.md", "docs/adr/0017-command-tables-from-cld-with-vcdu.md", "https://typewritten.org/Manual/DEC/VMS/5.4/mailhelp.html"]

[[feature]]
id = "local"
title = "SEND, READ, REPLY, FORWARD, DIRECTORY between users of one system"
status = "todo"
needs = ["/rms/indexed", "/utils/loginout", "/dcl/cld"]

[[feature]]
id = "folders"
title = "Folders in MAIL.MAI: NEWMAIL, MAIL, WASTEBASKET, MOVE, FILE, SELECT, EXTRACT, PURGE, COMPRESS"
status = "todo"
needs = ["#local"]

[[feature]]
id = "notify"
title = "New mail on node from user: a broadcast on the recipient's terminal, and the count at login"
status = "todo"
needs = ["#local", "/drivers/terminal#brkthru"]
note = "$BRKTHRU, PRD-0003 tier 1 #3"

[[feature]]
id = "delivery"
title = "Delivery into another user's mail file, as an image installed with SYSPRV"
status = "todo"
needs = ["#local", "/exec/installed-images", "/exec/privileges"]
note = "Until INSTALL, process.mar's KNOWN list could give it SYSPRV, as it does SET PASSWORD (ADR-0029)"

[[feature]]
id = "profile"
title = "The profile, VMSMAIL_PROFILE.DATA: SET FORWARD, PERSONAL_NAME, EDITOR, CC_PROMPT; @lists"
status = "todo"
needs = ["#local"]

[[feature]]
id = "editor"
title = "SEND/EDIT and REPLY/EDIT in EVE through callable TPU, or in EDT"
status = "todo"
needs = ["#local", "/rtl/tpushr"]

[[feature]]
id = "print"
title = "PRINT of messages to SYS$PRINT"
status = "todo"
needs = ["#local", "/utils/job-controller#print-queues"]

[[feature]]
id = "between-systems"
title = "NODE::USER between vaxpunk systems over TCP/IP, as DECnet's MAIL-11 did"
status = "todo"
needs = ["#local", "/net/tcpip-qio"]
note = "PRD-0002: mail between office VMS boxes; the protocol is its open question"

[[feature]]
id = "internet"
title = "SMTP%\"user@host\": Internet mail through TCP/IP Services' SMTP"
status = "todo"
needs = ["#local"]
note = "t8y:/net/smtp delivers it"

[[feature]]
id = "callable"
title = "Callable mail: MAIL$ routines in MAILSHR, for programs that send or read mail"
status = "todo"
needs = ["#local", "/exec/shareable-images"]

[[feature]]
id = "mime"
title = "MIME: attachments read and sent"
status = "todo"
needs = ["#local"]

[[needs]]
on = "/rms/indexed"
why = "MAIL.MAI and VMSMAIL_PROFILE.DATA are indexed files"

[[needs]]
on = "/rtl/cli"
why = "Its MAIL> commands are parsed with its own CLD table, as ADR-0017 says"

[[needs]]
on = "/utils/loginout"
why = "Users, their directories and the new mail count at login"

[[needs]]
on = "/net/smtp"
why = "SMTP% mail goes out through TCP/IP Services' SMTP"
soft = true

[[needs]]
fuzzy = "A mail transport between vaxpunk systems over TCP/IP: SMTP, or something closer to MAIL-11"
why = "PRD-0002 leaves the protocol for mail between office VMS boxes open"
+++

VMS MAIL is the mail program every VMS user has. `MAIL` opens a
`MAIL>` prompt; `SEND` asks `To:` and `Subj:` and takes the text from
the terminal or an editor, `READ` shows new messages, and `REPLY`,
`FORWARD`, `EXTRACT`, `PRINT` and `DIRECTORY` work on them. A user's
messages are in an indexed file, `MAIL.MAI`, in folders: `NEWMAIL`
until read, `MAIL` after, and `WASTEBASKET` for deleted ones until
`PURGE`. A message to a logged-in user rings their terminal with "New
mail on node X from Y", and `LOGINOUT` says how many are waiting. The
system profile, `VMSMAIL_PROFILE.DATA`, keeps each user's settings:
forwarding address, personal name, editor (EDT or EVE, called through
their callable interfaces) and the new mail count. Over DECnet,
`NODE::USER` addressed users on other systems through the MAIL-11
protocol, and TCP/IP Services added `SMTP%"user@host"`. Programs send
mail through callable MAIL, `MAILSHR`.

vaxpunk has no MAIL. PRD-0003 leaves mail to PRD-0002, whose goals
include "mail and chat between office VMS boxes" and whose open
question asks which protocol carries them; DECnet is dropped. What it
needs is mostly built or in Milestone 1: indexed files (PRD-0008), users
and their directories (`LOGINOUT`, `AUTHORIZE`), a CLD-based prompt
(ADR-0017 names `MAIL` as the example). Missing are `$BRKTHRU` for the
new mail message, a way for `MAIL.EXE` to write into other users' files
without giving everyone `SYSPRV`, and the protocol between systems.
