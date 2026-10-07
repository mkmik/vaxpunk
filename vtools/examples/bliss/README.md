# BLISS examples

BLISS-64 programs to compile, link and run with the vtools commands,
through [just](https://just.systems). [docs/vbliss.md](../../docs/vbliss.md)
describes what `vbliss` takes.

```sh
cd vtools/examples/bliss
just date               # builds the tools and SYSLIB.OLB, then compiles, links and runs date
just --dry-run date     # shows the commands without running them
```

The [Justfile](Justfile) holds the commands. `cargo test -p vrun` also runs
each example and checks its output against
[tests/examples/bliss](../../tests/examples/bliss), like the tests in
[tests/bliss](../../tests/bliss).
