$ ! DCLTEST.COM: checks DCL's symbols and command procedures.
$ ! @DCLTEST 3 "Two words" counts to 3, calls itself, and writes
$ ! DCLTEST: ok. @DCLTEST FAIL stops at the TYPE that fails.
$ IF P1 .EQS. "INNER" THEN GOTO INNER
$ IF P1 .EQS. "FAIL" THEN GOTO FAIL
$ N = 0
$ S = ""
$ LOOP:
$       N = N + 1
$       S = S + "x"
$       IF N .LT. P1 THEN GOTO LOOP
$ IF S .NES. "xxx" THEN GOTO BAD
$ IF 2 + 3 * 4 .NE. 14 .OR. (2 + 3) * 4 .NE. 20 THEN GOTO BAD
$ IF .NOT. (N .EQ. 3 .AND. -N .LT. 0) THEN GOTO BAD
$ IF P2 .NES. "Two words" THEN GOTO BAD
$ T := 'P2'
$ IF T .NES. "TWO WORDS" THEN GOTO BAD
$ @SYS$MANAGER:DCLTEST INNER 'N'
$ IF N .NE. 3 .OR. RESULT .NE. 4 THEN GOTO BAD ! the inner N is its own
$ WRITE SYS$OUTPUT "DCLTEST: ok, ", N, " and ", RESULT
$ EXIT
$ INNER:
$ N = P2 + 1
$ RESULT == N
$ EXIT
$ FAIL:
$ TYPE NOSUCH.TXT
$ WRITE SYS$OUTPUT "DCLTEST: not here"
$ BAD:
$ WRITE SYS$OUTPUT "DCLTEST: failed, N = ", N
$ EXIT 44
