+++
title = "Personas"
summary = "Security profiles a process or thread can create and assume: $PERSONA_ services"
aka = ["persona", "PSB", "persona security block", "$PERSONA_CREATE", "$PERSONA_ASSUME", "$PERSONA_DELETE", "$PERSONA_QUERY", "$PERSONA_MODIFY", "$PERSONA_DELEGATE", "$PERSONA_EXTENSION_LOOKUP", "per-thread security", "IMPERSONATE", "persona extensions", "NT persona"]
kind = "core"
intent = "aspiration"
research = "deferred"

[[feature]]
id = "psb"
title = "The security profile in a persona block, not in the PCB: UIC, privileges, rights, username"
status = "todo"

[[feature]]
id = "services"
title = "$PERSONA_CREATE, $PERSONA_ASSUME, $PERSONA_DELETE, $PERSONA_QUERY, $PERSONA_MODIFY"
status = "todo"
needs = [":psb"]

[[feature]]
id = "impersonate"
title = "IMPERSONATE, which was DETACH: create or assume another user's persona"
status = "todo"
needs = [":services"]

[[feature]]
id = "per-thread"
title = "Per-thread security: each kernel thread may run as its own persona"
status = "todo"
needs = [":psb", "//exec/kernel-threads"]

[[feature]]
id = "extensions"
title = "Persona extensions for other systems' identities, Windows NT's first"
status = "todo"
needs = [":services"]

[[needs]]
on = "//exec/privileges"
why = "A persona holds the privilege masks"

[[needs]]
on = "//exec/uic-protection"
why = "A persona holds the UIC every check uses"

[[needs]]
on = "//exec/rights-database"
why = "A persona holds the rights list"
soft = true
+++

From Alpha V7.2, a process's security profile (UIC, privileges,
rights list, username, account) is no longer part of the PCB but a
persona, kept in a persona security block. The `$PERSONA_` services
let a privileged server create a persona for a user, assume it while
it does that user's work, so every check sees the user and not the
server, and go back to its own. With per-thread security each kernel
thread may run as its own persona. The `IMPERSONATE` privilege,
the former `DETACH`, allows it. Persona extensions carry another
system's identity, such as a Windows NT user's for PATHWORKS and
Advanced Server.

vaxpunk keeps the profile in the PCB, as VMS did before V7.2:
`PCB$L_UIC`, the three privilege masks and `PCB$T_USERNAME`. Its
servers, `TELNETD` and the job controller to come, create processes
for users rather than acting for them.
