$ ! KEYS.COM: MAKE.COM's files with packed decimal keys: PD1, a string
$ ! key 0 with packed and descending packed alternate keys, PD2 a packed
$ ! primary key without duplicates, PD3 a descending one with them, each
$ ! given good and bad values; PKN a packed key with null keys, and
$ ! descending keys of other types; then files with collated keys. P1 is
$ ! where the FDL files and COLDATA.TXT are.
$ SET NOON
$ IN = P1
$ V1 = "0000005D/-5 D,0000000C/0 C,0000007C/7 C,0000012C/12 C,0123456C/123456 C,0000999D/-999 D"
$ V2 = "9999999C/9999999 C,9999999D/-9999999 D,0000005F/5 F,0000000F/0 F,0000000D/-0 D"
$ V3 = "0000005A/5 A,0000005B/-5 B,0000007E/7 E,0000001C/1 C,0000001D/-1 D,0000000B/-0 B"
$ VL = V1 + "," + V2 + "," + V3
$ BAD = "00000A5C/bad digit A,00000123/bad sign 3,F000000C/bad digit F,0000000A/0 A"
$ ! PD1: string key 0 (a label), packed key 1, descending packed key 2
$ CREATE/FDL='IN'PD1.FDL PD1.IDX
$ OPEN/READ/WRITE F PD1.IDX
$ LST = VL
$ GOSUB PUTALL
$ LST = BAD
$ GOSUB PUTALL
$ CLOSE F
$ ! PD2: packed primary key, no duplicates: are 0C, 0D, 0F one key?
$ CREATE/FDL='IN'PD2.FDL PD2.IDX
$ OPEN/READ/WRITE F PD2.IDX
$ LST = VL
$ GOSUB PUTALL
$ LST = BAD
$ GOSUB PUTALL
$ CLOSE F
$ ! PD3: descending packed primary key, duplicates
$ CREATE/FDL='IN'PD3.FDL PD3.IDX
$ OPEN/READ/WRITE F PD3.IDX
$ LST = VL
$ GOSUB PUTALL
$ CLOSE F
$ ! PKN: null packed key, descending high keys
$ CREATE/FDL='IN'PKN.FDL PKN.IDX
$ OPEN/READ/WRITE F PKN.IDX
$ RL = "00000C/apple /5,00000D/Banana/-5,00000F/cherry/0,00012C/apple /70000,00001D/zzzzzz/-1,00000A/      /2,00000B/APPLE /-70000"
$ N = 0
$ P1:
$ E = F$ELEMENT(N, ",", RL)
$ IF E .EQS. "," THEN GOTO P2
$ H = F$ELEMENT(0, "/", E)
$ DS = F$ELEMENT(1, "/", E)
$ V = F$INTEGER(F$ELEMENT(2, "/", E))
$ R = F$FAO("!4ZL!3* !6AS!17* ", N + 1, DS)
$ B = F$EXTRACT(0, 2, H)
$ R[32,8] = %X'B'
$ R[184,8] = %X'B'
$ B = F$EXTRACT(2, 2, H)
$ R[40,8] = %X'B'
$ R[192,8] = %X'B'
$ B = F$EXTRACT(4, 2, H)
$ R[48,8] = %X'B'
$ R[200,8] = %X'B'
$ R[104,32] = V
$ R[136,32] = V
$ R[168,16] = V
$ R[208,32] = V
$ WRITE F R
$ WRITE SYS$OUTPUT "PKN put ", E, " ", $STATUS
$ N = N + 1
$ GOTO P1
$ P2:
$ CLOSE F
$ ! COL1C, COL6C, COL8 and COL9: collated and descending collated keys,
$ ! with NCS's Multinational, Spanish and French sequences
$ CONVERT/FDL='IN'COL1.FDL 'IN'COLDATA.TXT COL1C.IDX
$ CONVERT/FDL='IN'COL6.FDL 'IN'COLDATA.TXT COL6C.IDX
$ CONVERT/FDL='IN'COL8.FDL 'IN'COLDATA.TXT COL8.IDX
$ CONVERT/FDL='IN'COL9.FDL 'IN'COLDATA.TXT COL9.IDX
$ EXIT
$ !
$ PUTALL:
$ N = 0
$ PA1:
$ E = F$ELEMENT(N, ",", LST)
$ IF E .EQS. "," THEN RETURN
$ H = F$ELEMENT(0, "/", E)
$ LB = F$ELEMENT(1, "/", E)
$ GOSUB MKPD
$ R = R + " " + F$FAO("!12AS", LB) + " rec"
$ WRITE F R
$ S = $STATUS
$ WRITE SYS$OUTPUT "PUT ", H, " ", LB, " status ", S
$ N = N + 1
$ GOTO PA1
$ !
$ ! MKPD: R = the 4 bytes whose hex is H
$ MKPD:
$ R = ""
$ J = 0
$ MK1:
$ B = F$EXTRACT(J*2, 2, H)
$ P = J*8
$ R['P',8] = %X'B'
$ J = J + 1
$ IF J .LT. 4 THEN GOTO MK1
$ RETURN
