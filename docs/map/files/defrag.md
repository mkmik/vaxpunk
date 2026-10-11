+++
title = "Disk defragmentation"
summary = "Moving files' extents together on a mounted volume: DFO and DFU"
aka = ["DFO", "Disk File Optimizer", "DFU", "Disk File Utility", "move file", "fragmentation"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/files/ods2"
why = "Rewrites files' maps to point at new, contiguous runs"

[[needs]]
on = "/files/fs-locking"
why = "Moves files while other processes use the volume"
+++

A Files-11 volume fragments as files grow a little at a time: a file's
map gets many retrieval pointers, extension headers follow, and free
space breaks into small runs. The XQP has a move-file function that copies
a file's blocks to new ones and switches its map while the file stays in
use. HP sold the Disk File Optimizer (DFO) on top of it, which reports
fragmentation and defragments a mounted volume; the freeware DFU does the
same and more. `BACKUP/IMAGE` to a fresh volume is the old way.

vaxpunk's volumes are at most 4,096 blocks and nothing is planned here.
