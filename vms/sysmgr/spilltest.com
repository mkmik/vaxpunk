$ ! SPILLTEST.COM: makes 70 versions of DKB0:[SPILL]V.TXT, more than one
$ ! directory record holds, so that they spill into a second. The name's
$ ! highest version is found once, every version once, and a version
$ ! deleted from each record, and one made below them all, are where
$ ! they should be. Writes SPILLTEST: ok, or what went wrong.
$ CREATE/DIRECTORY DKB0:[SPILL]
$ N = 0
$ MAKE:
$       N = N + 1
$       OPEN/WRITE F DKB0:[SPILL]V.TXT
$       CLOSE F
$       IF N .LT. 70 THEN GOTO MAKE
$ SPEC = "DKB0:[SPILL]*.*;0"
$ WANT = 1
$ AFTER = "ALL"
$ GOTO COUNT
$ ALL:
$ SPEC = "DKB0:[SPILL]*.*;*"
$ WANT = 70
$ AFTER = "CHANGE"
$ GOTO COUNT
$ CHANGE:
$ DELETE DKB0:[SPILL]V.TXT;70
$ DELETE DKB0:[SPILL]V.TXT;1
$ OPEN/WRITE F DKB0:[SPILL]V.TXT;1
$ CLOSE F
$ SPEC = "DKB0:[SPILL]V.TXT;0"
$ IF F$SEARCH(SPEC) .NES. "DKB0:[SPILL]V.TXT;69" THEN GOTO BAD
$ SPEC = "DKB0:[SPILL]*.*;*"
$ WANT = 69
$ AFTER = "OK"
$ GOTO COUNT
$ OK:
$ WRITE SYS$OUTPUT "SPILLTEST: ok"
$ EXIT
$ ! COUNT: counts the files SPEC names, which must be WANT, then goes to AFTER.
$ COUNT:
$ N = 0
$ NEXT:
$       IF F$SEARCH(SPEC) .EQS. "" THEN GOTO COUNTED
$       N = N + 1
$       GOTO NEXT
$ COUNTED:
$ IF N .NE. WANT THEN GOTO BAD
$ GOTO 'AFTER'
$ BAD:
$ WRITE SYS$OUTPUT "SPILLTEST: ", SPEC, " found ", N
