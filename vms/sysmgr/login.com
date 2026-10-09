$ ! LOGIN.COM: SYSTEM's own login procedure, in its login directory,
$ ! SYS$LOGIN. DCL runs it after SYLOGIN.COM, as LOGINOUT runs a user's
$ ! on VMS. What follows is for a terminal: a batch job skips it.
$ IF F$MODE() .NES. "INTERACTIVE" THEN EXIT
$ TT = F$GETDVI("SYS$INPUT", "DEVNAM")
$ WRITE SYS$OUTPUT F$FAO("!AS on !AS, !AS", F$GETJPI("", "PRCNAM"), TT, F$TIME())
