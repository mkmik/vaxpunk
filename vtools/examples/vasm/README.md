# vasm examples

ARM64 programs to assemble, link and run with the vtools commands, through
[just](https://just.systems):

```sh
cd vtools/examples/vasm
just hello              # builds the tools, then assembles, links and runs hello
just --dry-run library  # shows the commands without running them
```

The [Justfile](Justfile) holds the commands. `cargo test -p vrun` also runs
each example and checks its output against [tests/examples/vasm](../../tests/examples/vasm),
like the tests in [tests/run](../../tests/run).
