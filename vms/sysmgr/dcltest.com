$ ! DCLTEST.COM: checks DCL's symbols and command procedures.
$ ! @DCLTEST 3 "Two words" counts to 3, calls itself, runs CLITEST as a
$ ! foreign command and as GREET, which SET COMMAND adds from DCLTEST.CLD,
$ ! checks block IFs, lexical functions, ON, a file it writes and reads
$ ! and data lines, and writes DCLTEST: ok. @DCLTEST FAIL stops at the TYPE that
$ ! fails.
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
$ ! A block IF, THEN on a line of its own, another inside it.
$ IF N .EQ. 3
$ THEN
$       IF N .EQ. 4 THEN
$               GOTO BAD
$       ELSE
$               B = "else"
$       ENDIF
$ ELSE
$       GOTO BAD
$ ENDIF
$ IF B .NES. "else" THEN GOTO BAD
$ ! Lexical functions.
$ IF F$LENGTH(S) .NE. 3 .OR. F$EXTRACT(1, 1, "abc") .NES. "b" THEN GOTO BAD
$ IF F$LOCATE("c", "abc") .NE. 2 .OR. F$ELEMENT(1, ",", "a,b") .NES. "b" THEN GOTO BAD
$ IF F$EDIT("  a  b ", "COMPRESS,TRIM,UPCASE") .NES. "A B" THEN GOTO BAD
$ IF F$TYPE(N) .NES. "INTEGER" .OR. F$TYPE(NOSUCH) .NES. "" THEN GOTO BAD
$ IF F$MODE() .NES. "INTERACTIVE" .OR. F$ENVIRONMENT("DEPTH") .NE. 1 THEN GOTO BAD
$ IF F$FAO("!3UL!AS", 7, "x") .NES. "  7x" .OR. %X10 .NE. 16 THEN GOTO BAD
$ IF F$GETJPI("", "PRCNAM") .NES. "SYSTEM" THEN GOTO BAD
$ IF F$TRNLNM("SYS$SYSTEM") .NES. "SYS$SYSROOT:[SYSEXE]" THEN GOTO BAD
$ IF F$PARSE("X", "SYS$MANAGER:.COM", , "TYPE") .NES. ".COM" THEN GOTO BAD
$ ! ON: TYPE's error, RMS-E-FNF, runs the ON command, once.
$ ON ERROR THEN ST = $STATUS
$ TYPE NOSUCH.TXT
$ IF ST .NE. %X10018292 .OR. F$ENVIRONMENT("ON_SEVERITY") .NES. "ERROR" THEN GOTO BAD
$ ! A file on the ramdisk: CREATE's data line, a record WRITE adds, then
$ ! read to its end; and READ's data line.
$ CREATE DCLTEST.TMP
one
$ OPEN/APPEND OUT DCLTEST.TMP
$ WRITE OUT "two ", N
$ CLOSE OUT
$ OPEN IN DCLTEST.TMP
$ READ IN L
$ READ IN L2
$ READ/END_OF_FILE=EOF IN L3
$ GOTO BAD
$ EOF:
$ CLOSE IN
$ IF L .NES. "one" .OR. L2 .NES. "two 3" .OR. F$TRNLNM("IN") .NES. "" THEN GOTO BAD
$ DELETE DCLTEST.TMP;*
$ READ SYS$INPUT L
data
$ IF L .NES. "data" THEN GOTO BAD
$ CT := $CLITEST
$ CT one "Two" 3
$ SET COMMAND SYS$MANAGER:DCLTEST
$ GREET world
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
