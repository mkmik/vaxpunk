+++
title = "Callable TPU"
summary = "TPU$TPUSHR: TPU as a library, TPU$TPU, TPU$INITIALIZE, TPU$EXECUTE_COMMAND"
aka = ["TPU$TPUSHR", "TPUSHR", "TPU$TPU", "TPU$INITIALIZE", "TPU$EDIT", "callable EDT", "EDT$EDIT"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"
refs = ["docs/prd/0006-tpu-and-eve.md"]

[[needs]]
on = "/utils/tpu"
why = "It is TPU's own code, as a shareable image"

[[needs]]
on = "/exec/shareable-images"
why = "A shareable image that calls another, LIBRTL.EXE (ADR-0028 can't yet)"
+++

On VMS, TPU is a shareable image, `TPU$TPUSHR.EXE`, with a callable
interface: a program calls `TPU$TPU` with a command line, or
`TPU$INITIALIZE`, `TPU$EXECUTE_COMMAND` and `TPU$CLEANUP` to embed the
editor, as MAIL, LSE and other utilities do for their editing. EDT has
the same kind of interface, `EDT$EDIT`.

vaxpunk's TPU is one image, `TPU.EXE`. PRD-0006 leaves out the callable
interface and `TPU$TPUSHR` until a shareable image may call another
one.
