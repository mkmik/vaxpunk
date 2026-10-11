+++
title = "LDAP"
summary = "LDAP client API, and logins checked against a directory (LDAP ACME)"
aka = ["LDAP", "ldap_*", "LDAP ACME", "SYS$ACM", "external authentication"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "//net/tcpip-qio"
why = "Talks to a directory server on port 389"

[[needs]]
on = "//net/ssl"
why = "LDAP over TLS"
soft = true

[[needs]]
on = "//utils/loginout"
why = "External authentication checks a user's password against the directory"
+++

OpenVMS has an LDAP client API, the `ldap_*` routines, and, in later releases,
the `SYS$ACM` service with an LDAP ACME agent, so LOGINOUT can check a
user's password against a directory such as Active Directory, the user
still having a UAF record for UIC and privileges.

vaxpunk has none.
