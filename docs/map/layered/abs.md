+++
title = "Archive Backup System"
summary = "ABS and SLS before it: scheduled backups, tape pools and robots, catalogs"
aka = ["ABS", "Archive Backup System", "SLS", "Storage Library System", "MDMS", "Media and Device Management Services"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://h41379.www4.hpe.com/openvms/storage/slspage.html", "https://h41379.www4.hpe.com/openvms/storage/smstape_matrix.html"]

[[needs]]
on = "/utils/backup"
why = "Save sets are written by VMS BACKUP"

[[needs]]
on = "/drivers/tape"
why = "Tape drives and the robots that load them"

[[needs]]
on = "/utils/job-controller"
why = "Backups run on a schedule as batch jobs"

[[needs]]
on = "/rms/indexed"
why = "Catalogs of which file is on which tape"

[[needs]]
fuzzy = "Media management: tape pools, volume labels, robot control, off-site rotation"
why = "Knowing which tape to load and where it is"
+++

Sites with many disks and tape libraries didn't run `BACKUP` by hand.
SLS, the Storage Library System, scheduled backups, drove tape robots,
managed pools of tapes and kept catalogs of which file was saved on
which volume, so a restore could find it. ABS, the Archive Backup
System, replaced it, adding clients for Windows and Unix, with media
and device management (MDMS) as a separate layer under it. SLS stayed
on VAX and Alpha in maintenance.

On vaxpunk it is an aspiration. vaxpunk has `BACKUP` and save sets
([t8y:/utils/backup](../utils/backup.md)) but no tape driver.
