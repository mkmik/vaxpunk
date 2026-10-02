# MACRO-32 examples

VAX MACRO-32 programs to compile, link and run with the vtools commands,
through [just](https://just.systems). [docs/macro32.md](../../docs/macro32.md)
describes what `vmacro` takes.

```sh
cd vtools/examples/macro32
just hello              # builds the tools, then compiles, links and runs hello
just --dry-run calls    # shows the commands without running them
```

The [Justfile](Justfile) holds the commands. `cargo test -p vrun` also runs
each example and checks its output against
[tests/examples/macro32](../../tests/examples/macro32), like the tests in
[tests/macro32](../../tests/macro32).
