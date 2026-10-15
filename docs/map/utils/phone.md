+++
title = "PHONE"
summary = "Live split-screen chat between users, on one system or between systems"
aka = ["PHONE", "PHONE.EXE", "DIAL", "ANSWER", "HANGUP", "FACSIMILE", "switch hook", "chat", "talk"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/prd/0002-networking.md", "vms/sysexe/phone.mar", "vms/cld/phone.cld", "https://typewritten.org/Manual/DEC/VMS/5.4/helplib.hlb/PHONE.html"]

[[feature]]
id = "local"
title = "PHONE user: ring, ANSWER, a window per caller, HANGUP, on one system"
status = "done"
note = "PHN$username mailboxes, as VMS's; PHONE is in process.mar's KNOWN list with OPER, PRMMBX, SYSNAM and WORLD"
needs = ["/exec/mailboxes", "/drivers/terminal"]

[[feature]]
id = "ring"
title = "The ring: a broadcast on the callee's terminal, repeated until answered"
status = "done"
needs = ["#local", "/drivers/terminal#brkthru"]
note = "$BRKTHRU of each of the user's terminals, BRK$C_DEVICE and BRK$C_PHONE, every 10 seconds, as VMS's"

[[feature]]
id = "directory"
title = "DIRECTORY: who is logged in and can be called"
status = "done"
needs = ["#local", "/exec/information-services"]

[[feature]]
id = "commands"
title = "HOLD, UNHOLD, FACSIMILE of a file, MAIL to an absent user, conference calls, HELP"
status = "todo"
note = "REJECT is done; HELP says the help library is missing, as VMS's does without PHONEHELP.HLB"
needs = ["#local"]

[[feature]]
id = "between-systems"
title = "PHONE NODE::USER between vaxpunk systems over TCP/IP"
status = "todo"
needs = ["#local", "/net/tcpip-qio"]
note = "PRD-0002: chat between office VMS boxes; the protocol is its open question"

[[needs]]
on = "/exec/mailboxes"
why = "The local parties' processes talk through mailboxes"

[[needs]]
on = "/drivers/terminal"
why = "Reads each key as it is typed and paints the windows"

[[needs]]
on = "/rtl/smg"
why = "The split screen and its viewports"
soft = true

[[needs]]
on = "/utils/mail"
why = "PHONE's MAIL command leaves a message for someone who doesn't answer"
soft = true

[[needs]]
fuzzy = "A call protocol between vaxpunk systems over TCP/IP, in place of DECnet's PHONE object"
why = "PRD-0002 leaves the protocol for chat between office VMS boxes open"
+++

`PHONE` is VMS's chat program. `PHONE SMITH` rings user SMITH, whose
terminal shows a broadcast that someone is phoning, repeated until
`PHONE ANSWER` or `REJECT`. The screen splits into a window per party,
and each character appears on the other side as it is typed. Commands
start with the switch hook, `%`: `DIAL` adds a caller for a conference,
`HOLD`, `HANGUP`, `DIRECTORY` lists who can be called, `FACSIMILE`
sends a file's text into the conversation, and `MAIL` leaves a message
for someone who isn't there. With DECnet, `PHONE NODE::USER` called
users on other systems.

vaxpunk's PHONE does calls on one system, as VMS's does: each user has
a mailbox, `PHN$username`, which whoever needs it first makes; a ring is
a message there and a broadcast on the user's terminals every 10
seconds; the screen, the commands and the messages are VMS's. PHNTEST,
in the boot test, logs JOE in on a pseudo terminal and answers SYSTEM's
call. HOLD, FACSIMILE, MAIL and conference calls are still missing.

PHONE between systems is one of the era-appropriate office
applications Marko wants between VMS boxes, which PRD-0002 names as a
goal for TCP/IP to unblock ("chat between office VMS boxes") while
leaving the protocol open. DECnet is dropped, so the call between
systems needs a TCP/IP protocol of vaxpunk's own or an existing one.
