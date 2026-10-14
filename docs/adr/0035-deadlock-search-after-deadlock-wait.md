# ADR-0035 — A lock request is searched for a deadlock once it has waited DEADLOCK_WAIT seconds, as on VMS

Oct 14, 2026 · @Marko Mikulicic

Accepted. The lock manager searches for a deadlock from a request that
has waited `DEADLOCK_WAIT` seconds, 10 by default, as VMS does, not from
every request as soon as it has to wait. This replaces decision 5 of
[ADR-0030](0030-lock-manager-in-the-executive.md); its other decisions
stand.

## Context

ADR-0030 chose to search at once, when a request is queued: a cycle can
only close then, and `LOCKTEST` wouldn't have to wait 10 s to see one.
Building it showed what that costs.

VMS's search goes from the waiting request to the processes that hold
the locks blocking it, to those processes' own waiting requests, and so
on, and a deadlock is a path back to the first request's process. So a
process that asks for a lock its own other lock blocks is deadlocked
with itself. DEC's `DEADLOCK.MAR` says as much: one process locking the
same resource twice is senseless, *unless it is multi-threaded*, with
ASTs. Such a process often releases the other lock itself a moment
later: `LOCKTEST` takes two protected reads, asks for an exclusive lock
with `$ENQ`, and dequeues the reads. On VMS the exclusive lock is
granted, since nothing is searched for 10 s. With a search at once it
fails with `SS$_DEADLOCK`. The same holds across processes: two
processes whose requests cross with `$ENQ` and that let go of one lock
from an AST are a deadlock only to a search that doesn't wait.

`DEADLOCK_WAIT` is a SYSGEN parameter, `LCK$GL_WAITTIME`, in seconds;
0 turns the search off.

## Decision

1. **A request that has waited `DEADLOCK_WAIT` seconds is searched from,
   once.** `LCK$GL_WAITTIME` is a cell in `sysenqdeq.mar`, 10, until
   there is a SYSGEN to set it. Each lock whose request waits counts the
   seconds down; at 0 the search starts from it. If it finds no cycle,
   the request isn't searched again: a cycle it ends up in later is closed
   by a newer request, which is searched from in its turn, as on VMS.
2. **Once a second, while a request is left to search from.** The first
   request that waits puts a timer queue entry of the lock manager's on
   the queue, a second ahead. Each time it is due, `LCK$SEARCH` counts
   down and searches, and the entry goes back on the queue while a
   request still counts down. No request waiting, no timer.
3. **What a request waits for.** A lock blocks a request, converting or
   waiting, if it holds a mode the requested mode isn't compatible with,
   or asks for one ahead of it in the queues: conversions before new
   requests, each in order. That is what keeps it from being granted, and
   what `$GETLKI`'s `LKI$_BLOCKING` and `LKI$_BLOCKEDBY` list.
4. **The request searched from is the one that fails**, with
   `SS$_DEADLOCK` in its status block: a new lock is not granted and
   goes, a conversion keeps its granted mode. Which request VMS fails is
   undocumented, so no program may depend on it.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Search at once, when the request is queued (ADR-0030) | Fails requests VMS grants: a process that waits for its own lock, or crosses another's, and lets go of one itself |
| Search at once, but ignore cycles through one process only | Still fails the two-process case, and a process really deadlocked with itself would hang instead of getting `SS$_DEADLOCK` |
| VMS's lock timeout queue, each request due at its own time | Exact to the tick, but a queue to keep in order; the 10 s are a heuristic, and a second either way changes nothing |

## Consequences

**What gets harder.**
- A deadlock takes `DEADLOCK_WAIT` seconds to be found. `LOCKTEST`'s
  takes about 10 s at boot, in the background of the startup phase.
- ponytail: while a request waits, the lock ID table is scanned once a
  second; a list of the waiting requests if locks get many.
- ponytail: `DEADLOCK_WAIT` is a cell, not a SYSGEN parameter, since
  there is no SYSGEN yet.

**What stays easy.**
- Programs that wait for their own locks, or cross another process's
  and let go, behave as on VMS.
- No search runs while nothing waits, and none runs on a request granted
  within the wait, which is nearly all of them.
