+++
title = "VAX Notes"
summary = "Computer conferencing: conferences of topics and replies, local or remote"
aka = ["VAX Notes", "DEC Notes", "NOTES", "notesfile", "notebook", "conference", "NOTES$SERVER", "NOTES$", "dec-notes", "DDIS"]
kind = "layered"
intent = "aspiration"
research = "outline"
refs = ["https://openbookproject.net/tutorials/kjcole/vax-notes.html", "https://ftp.math.utah.edu/pub/tex/bib/idx/dectechj/1/6/117_124.html", "https://bitsavers.org/pdf/dec/dtj/dtj_v01-06_feb1988.pdf"]

[[feature]]
id = "local"
title = "Conferences on this system: OPEN, DIRECTORY, READ, WRITE and REPLY over topics and their replies"
status = "todo"
needs = ["/rms/indexed", "/rms/sharing", "/rtl/cli"]
note = "A whole conference is one indexed file; its records are DDIS, a type-length-value encoding, so new fields need no new format. The real layout isn't published"

[[feature]]
id = "notebook"
title = "The notebook: ADD ENTRY, the conferences a user follows and what they have seen, for NEXT UNSEEN"
status = "todo"
needs = ["#local"]
note = "Stored in the same format as a conference, through the same routines"

[[feature]]
id = "moderation"
title = "CREATE, moderators, members and keywords: SET MODERATOR, ADD MEMBER, ADD KEYWORD"
status = "todo"
needs = ["#local", "/exec/uic-protection"]

[[feature]]
id = "screen"
title = "The screen interface, written in TPU and run through callable TPU"
status = "todo"
needs = ["#local", "/rtl/tpushr", "/drivers/terminal"]

[[feature]]
id = "callable"
title = "NOTES$object_operation(context, inputs, outputs) on NOTEFILE, CLASS, ENTRY, KEYWORD, NOTE, PROFILE and USER, with item lists"
status = "todo"
needs = ["#local", "/exec/shareable-images"]
note = "Gilbert 1988: every operation is BEGIN, END, ADD, DELETE, GET or MODIFY of an object; the screen interface and the server are both built on it"

[[feature]]
id = "remote"
title = "Conferences on other systems through NOTES$SERVER, a network object that serves many users from one multithreaded process"
status = "todo"
needs = ["#callable", "/net/task-to-task#incoming", "/net/task-to-task#non-transparent", "/net/task-to-task#proxies"]
note = "Each call's operation, context and item lists go over as one DDIS record and come back the same way; repeated calls such as NOTE_GET fill large replies, and the client asks for more. DEC Notes later also ran over TCP, port 3333 (dec-notes), through UCX's auxiliary server"

[[feature]]
id = "mail"
title = "Topics and replies sent on by MAIL, and mail filed into a conference"
status = "todo"
needs = ["#local", "/utils/mail#callable"]

[[needs]]
on = "/net/task-to-task"
why = "Remote conferences are read through a network object, as on DECnet"
soft = true
+++

VAX Notes is DEC's conferencing system from the mid-1980s. A conference
is a file of topics, each with its replies, stored once and read by
everyone who subscribes, unlike mail. Each user's notebook lists the
conferences they follow and what they have seen, so `NEXT UNSEEN` walks
through what is new; moderators manage a conference, and keywords
group its topics. Inside DEC thousands of conferences carried the
company's technical discussion, and DECUServe ran public ones.

It descends from PLATO Notes through K-NOTES (1980, VAX PL/I) and
NOTES-11 (RSTS/E, then RSX and VMS). Those opened remote conferences
through RMS over DECnet, one network round trip per record. VAX Notes
added a server: the client sends a whole call of its callable
interface, the server runs it next to the file and sends back only the
answer, about ten times faster (Gilbert, Digital Technical Journal 6,
1988). The server is one process for many users, each request with its
own stack, switched at every I/O.

On vaxpunk it is an aspiration. It is the kind of chat between office
VMS boxes [PRD-0002](../../prd/0002-networking.md) wants to unblock.
Remote conferences fit t8y:/net/task-to-task, since `NOTES$SERVER` was
a DECnet object; that wire is vaxpunk's own, so it talks to vaxpunk
systems only.
