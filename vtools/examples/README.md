# Examples

Programs to assemble, link and run with the vtools commands, through
[just](https://just.systems):

```sh
cd vtools/examples
just hello              # builds the tools, then assembles, links and runs hello
just --dry-run library  # shows the commands without running them
just check              # checks every example against tests/examples
```

The [Justfile](Justfile) holds the commands. `cargo test -p vrun` also runs
each example and checks its output against [tests/examples](../tests/examples),
like the tests in [tests/run](../tests/run).
