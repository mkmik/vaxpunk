! How fixtures/vms-ods5.img and vms-ods2.img were made: VMS initializes
! and fills a volume of each level (run-vms.py samples.dcl, then take
! disk1.img and disk2.img). The log keeps DIRECTORY/FULL and DUMP/HEADER
! of every file, to compare with what ods reads.
SHOW DEVICE D
! ---------------- ODS-5 volume on DKA0 (disk1.img) ----------------
SET PROCESS/PARSE_STYLE=EXTENDED
INITIALIZE/STRUCTURE_LEVEL=5 DKA0: ODS5TEST
MOUNT DKA0: ODS5TEST
SHOW DEVICE/FULL DKA0:
SET DEFAULT DKA0:[000000]
CREATE/DIRECTORY [.MixedCase]
CREATE/DIRECTORY [.dir^.with^.dots]
CREATE lower.txt
hello from lower.txt
@@CTRLZ
CREATE MiXeD.CaSe
Mixed case name
@@CTRLZ
CREATE a^ file^ with^ spaces.txt
spaces in the name
@@CTRLZ
CREATE long_name_abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij.dat
74 character name
@@CTRLZ
CREATE version.txt
version 1
@@CTRLZ
CREATE version.txt
version 2
@@CTRLZ
CREATE version.txt
version 3
@@CTRLZ
CREATE café.txt
Latin-1 e-acute (0xE9) in the name
@@CTRLZ
CREATE plain.TXT
plain
@@CTRLZ
CREATE [.MixedCase]Inner.File.txt
in a mixed case subdirectory, with two dots in the name
@@CTRLZ
CREATE [.dir^.with^.dots]dotted.txt
in a directory with dots
@@CTRLZ
DIRECTORY/FULL [...]
DUMP/HEADER/BLOCK=COUNT:0 [...]*.*;*
ANALYZE/DISK_STRUCTURE DKA0:
SET DEFAULT SYS$SYSDEVICE:[000000]
DISMOUNT DKA0:
! ---------------- ODS-2 volume on DKA100 (disk2.img) ----------------
SET PROCESS/PARSE_STYLE=TRADITIONAL
INITIALIZE DKA100: ODS2TEST
MOUNT DKA100: ODS2TEST
SHOW DEVICE/FULL DKA100:
SET DEFAULT DKA100:[000000]
CREATE/DIRECTORY [.SUBDIR]
CREATE HELLO.TXT
Hello from ODS-2
@@CTRLZ
CREATE NOTES.DAT
line 1
line 2
@@CTRLZ
CREATE NOTES.DAT
version 2
@@CTRLZ
CREATE [.SUBDIR]NESTED.TXT
nested file
@@CTRLZ
DIRECTORY/FULL [...]
DUMP/HEADER/BLOCK=COUNT:0 [...]*.*;*
ANALYZE/DISK_STRUCTURE DKA100:
SET DEFAULT SYS$SYSDEVICE:[000000]
DISMOUNT DKA100:
SHOW DEVICE D
