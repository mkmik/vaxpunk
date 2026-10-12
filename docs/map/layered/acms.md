+++
title = "ACMS"
summary = "DEC's TP monitor: menus, tasks, forms exchanges, servers, queued tasks"
aka = ["ACMS", "VAX ACMS", "Application Control and Management System", "ADU", "ACMSGEN", "ACMSMGR", "QTI", "TP monitor", "ACMSDI", "TP Desktop Connector", "TP Web Connector"]
kind = "layered"
intent = "aspiration"
research = "outline"
refs = ["https://docs.vmssoftware.com/vsi-acms-concepts-and-design-guide/", "https://vmssoftware.com/docs/VSI_ACMS_GET_START_GUIDE.PDF", "https://docs.vmssoftware.com/vsi-acms-managing-applications/", "https://h41379.www4.hpe.com/doc/721final/6609/6609pro_015.html", "https://h41379.www4.hpe.com/doc/721final/6609/6609pro_007.html", "https://ftp.mirrorservice.org/sites/www.bitsavers.org/pdf/dec/dtj/dtj_v03-01_1991.pdf"]

[[feature]]
id = "adu"
title = "ADU: task, task group, menu and application definitions, built into databases"
status = "todo"
note = "Definitions compile into files or into CDD (files since ACMS V4.3); workspace records stay in CDD"
needs = ["/layered/cdd#records"]

[[feature]]
id = "runtime"
title = "The run-time system: ACC, TSC, Command Processes, an EXC per application"
status = "todo"
needs = ["#adu", "/exec/processes", "/exec/asts", "/exec/mailboxes"]

[[feature]]
id = "menus-sign-in"
title = "Terminal sign-in and menus, checked against SYSUAF and ACMS's user file"
status = "todo"
needs = ["#runtime", "/utils/authorize", "/drivers/terminal"]

[[feature]]
id = "procedure-servers"
title = "Procedure servers: step procedures in any language, started by the EXC"
status = "todo"
needs = ["#runtime", "/exec/calling-standard", "/exec/image-activator"]

[[feature]]
id = "dcl-servers"
title = "DCL servers: processing steps that run DCL commands and images"
status = "todo"
needs = ["#runtime", "/dcl/interpreter"]

[[feature]]
id = "workspaces"
title = "Task, group, user and system workspaces passed between steps"
status = "todo"
needs = ["#runtime", "/exec/global-sections"]

[[feature]]
id = "exchange-decforms"
title = "Exchange steps on DECforms: SEND, RECEIVE, TRANSCEIVE"
status = "todo"
needs = ["#runtime", "/layered/decforms#form-manager"]

[[feature]]
id = "exchange-tdms"
title = "Exchange steps on TDMS requests, for older applications"
status = "todo"
needs = ["#runtime", "/layered/tdms"]

[[feature]]
id = "transactions"
title = "Transactions in tasks across Rdb, DBMS and RMS, through DECdtm"
status = "todo"
needs = ["#procedure-servers", "/exec/decdtm#services", "/rms/journaling#recovery-units"]

[[feature]]
id = "queued-tasks"
title = "Task queues: ACMS$QUEUE_TASK and the Queued Task Initiator"
status = "todo"
needs = ["#transactions", "/rms/indexed", "/exec/lock-manager"]

[[feature]]
id = "systems-interface"
title = "Systems Interface: agents sign users in and call tasks (ACMS$SIGN_IN)"
status = "todo"
needs = ["#runtime"]

[[feature]]
id = "audit"
title = "Audit Trail Logger, Software Event Logger, and their report utilities"
status = "todo"
needs = ["#runtime", "/rms/sequential"]

[[feature]]
id = "management"
title = "ACMSGEN parameters, ACMS operator commands; UDU, DDU and AAU files"
status = "todo"
needs = ["#runtime", "/rms/indexed"]

[[feature]]
id = "distributed"
title = "Front ends doing the forms, back ends running the tasks, on other nodes"
status = "todo"
needs = ["#exchange-decforms", "/net/tcpip-qio"]
note = "ACMS's remote access option runs over DECnet, which vaxpunk drops; TCP/IP would replace it"

[[feature]]
id = "connectors"
title = "TP Desktop Connector and TP Web Connector clients"
status = "todo"
needs = ["#systems-interface", "/net/tcpip-qio"]

[[needs]]
on = "/layered/decforms"
why = "The forms of exchange steps; TDMS for older applications"

[[needs]]
on = "/layered/cdd"
why = "Record definitions for workspaces; the development kit needs it"

[[needs]]
on = "/exec/processes"
why = "Each part is a process: ACC, TSC, CPs, EXCs, servers, QTI, loggers"

[[needs]]
on = "/exec/asts"
why = "The CP and the EXC serve many users each, threaded by ASTs"

[[needs]]
on = "/exec/mailboxes"
why = "Messages between agents, the EXC and its servers"

[[needs]]
on = "/exec/global-sections"
why = "Workspace pools shared between processes; it raises GBLSECTIONS"

[[needs]]
on = "/exec/lock-manager"
why = "Task queues and shared files; the QTI needs SYSLCK"

[[needs]]
on = "/exec/decdtm"
why = "Distributed transactions in tasks and in task queues"

[[needs]]
on = "/exec/logical-names"
why = "Applications, forms and escape-unit images are found by logical name"

[[needs]]
on = "/exec/shareable-images"
why = "Server images, form images and the agents' interface"

[[needs]]
on = "/exec/condition-handling"
why = "Exception handling in tasks; procedures signal their errors"

[[needs]]
on = "/exec/privileges"
why = "Its processes use SYSPRV, SYSLCK, CMKRNL, SYSNAM; operators need OPER"

[[needs]]
on = "/exec/rights-database"
why = "Rights identifiers guard tasks and management commands"
soft = true

[[needs]]
on = "/exec/quotas"
why = "ACMSPARAM.COM sets each of its processes' quotas"
soft = true

[[needs]]
on = "/rms/indexed"
why = "Task queue files, and its user, device and application files"

[[needs]]
on = "/utils/authorize"
why = "Users sign in with their SYSUAF user name and password"
+++

ACMS, the Application Control and Management System, is DEC's
transaction processing monitor for VMS. An application is written in its
own definition language: tasks made of exchange steps (a form on the
user's terminal), processing steps (a procedure in a server, or DCL) and
block steps that group them into a transaction; task groups; menus; and
an application definition that says who may run what. ADU compiles them
into databases. At run time the Central Controller (ACC) starts the
Terminal Subsystem Controller, whose Command Processes sign users in and
show menus. Each application has an Application Execution Controller
(EXC) that runs the task definitions, hands out workspaces and sends
processing steps to a pool of single-threaded procedure servers. The CP
and the EXC are multithreaded with ASTs, so a few processes serve many
terminals and the database sees few users. Tasks can be queued for the
Queued Task Initiator, and a task's transaction spans Rdb, DBMS and RMS
through DECdtm. VSI sells it today.

On vaxpunk it is an aspiration, and the spine of Marko's signature
application idea, [the home-office inventory](home-inventory.md) on
DECforms, CDD and RMS indexed files. Nothing is built. Most of what it
needs from the base system is on the map: processes, ASTs, mailboxes,
global sections, the lock manager
([ADR-0030](../../adr/0030-lock-manager-in-the-executive.md)),
DECdtm and RMS journaling. The manuals don't say how its processes talk
to each other; mailboxes and global sections are the likely means.
