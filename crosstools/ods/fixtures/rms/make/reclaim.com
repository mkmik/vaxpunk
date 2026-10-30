$ ! RECLAIM.COM: MAKE.COM's files for CONVERT/RECLAIM. RC is DATA.TXT with
$ ! one-block buckets, every third record from K0000150 to K0000450 deleted
$ ! (RC1), then reclaimed (RC2), then 120 records put back into the freed
$ ! buckets (RC4). RCB has 300 fixed records and two index levels: the
$ ! first buckets, a whole level 1 index bucket's and the last bucket's
$ ! records deleted (RCB1), reclaimed (RCB2), and 71 put back (RCB3). P1
$ ! is where the FDL files and DATA.TXT are.
$ SET NOON
$ IN = P1
$ CONVERT/FDL='IN'RC.FDL 'IN'DATA.TXT RC.IDX
$ OPEN/READ/WRITE F RC.IDX
$ I = 50
$ D1:
$ K = F$FAO("K!7ZL", I*3)
$ READ/KEY="''K'"/DELETE F R
$ I = I + 1
$ IF I .LE. 150 THEN GOTO D1
$ CLOSE F
$ COPY RC.IDX RC1.IDX
$ CONVERT/RECLAIM/STATISTICS RC.IDX
$ COPY RC.IDX RC2.IDX
$ OPEN/READ/WRITE F RC.IDX
$ I = 300
$ P1:
$ WRITE F F$FAO("K!7ZL OMEGA new record number !UL", I, I)
$ I = I + 1
$ IF I .LT. 360 THEN GOTO P1
$ I = 700
$ P2:
$ WRITE F F$FAO("K!7ZL OMEGA new record number !UL", I, I)
$ I = I + 1
$ IF I .LT. 760 THEN GOTO P2
$ CLOSE F
$ COPY RC.IDX RC4.IDX
$ OPEN/WRITE O RCB.TXT
$ I = 1
$ T1:
$ WRITE O F$FAO("!30AS!30AS!30AS!10AS", F$FAO("K!7ZL", I), F$FAO("S!7ZL", I), F$FAO("D!7ZL", I/10), "rest")
$ I = I + 1
$ IF I .LE. 300 THEN GOTO T1
$ CLOSE O
$ CONVERT/FDL='IN'RCB.FDL RCB.TXT RCB.IDX
$ OPEN/READ/WRITE F RCB.IDX
$ I = 1
$ T2:
$ K = F$FAO("K!7ZL", I)
$ READ/KEY="''K'"/DELETE F R
$ I = I + 1
$ IF I .EQ. 13 THEN I = 40
$ IF I .EQ. 161 THEN I = 291
$ IF I .LE. 300 THEN GOTO T2
$ CLOSE F
$ COPY RCB.IDX RCB1.IDX
$ CONVERT/RECLAIM/STATISTICS RCB.IDX
$ COPY RCB.IDX RCB2.IDX
$ OPEN/READ/WRITE F RCB.IDX
$ I = 50
$ T3:
$ WRITE F F$FAO("!30AS!30AS!30AS!10AS", F$FAO("K!7ZL", I), F$FAO("S!7ZL", I), F$FAO("D!7ZL", I/10), "again")
$ I = I + 1
$ IF I .LE. 120 THEN GOTO T3
$ CLOSE F
$ COPY RCB.IDX RCB3.IDX
