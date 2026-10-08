# ADR-0008 — On a halt the root task prints a line, and the host powers QEMU off

Oct 2, 2026 · @Marko Mikulicic

Accepted. When the system halts, the root task prints a fixed line on the
console, `%PAL-I-POWEROFF`, and stops. `scripts/serial-filter.py`, which
already reads every byte of the console, sees the line and ends QEMU with
SIGTERM. This works the same under TCG and under HVF (`--hvf`), so
semihosting goes away.

## Context

vaxpunk should run under Hypervisor.framework as well as TCG, as an option:
`run-qemu.sh --hvf` and `cargo run -p boot -- --hvf`. TCG stays the
default, because it runs the Cortex-A57 the kernel is built for on every
host, Linux CI included. HVF runs the guest on the Mac's own CPU, which is
much faster. Today `--hvf` is best effort (see the README): the kernel is
built for a Cortex-A57 while HVF offers only `-cpu host`, and the counter
runs at 24 MHz instead of 62.5 MHz.

One thing that doesn't work under HVF is the halt. `LOGOUT` deletes
SYSTEM, the swapper prints `%EXEC-I-LOGOUT` and halts, and the root task
calls `poweroff()` (`pal/src/main.c`). That issues semihosting's
`SYS_EXIT`, `HLT #0xF000` with `x0 = 0x18`, and `run-qemu.sh` passes
`-semihosting-config enable=on,target=native,userspace=on` so QEMU accepts
it from EL0.

Semihosting works only because TCG translates every guest instruction and
can treat that `HLT` specially. Under HVF the guest runs on the real CPU,
with halting debug off, so `HLT` is an undefined instruction. The
exception goes to seL4 at EL1, never to QEMU. seL4 reports it as a fault
of the root task, and QEMU keeps running until the user types Ctrl-A x.

The usual way to power off is PSCI `SYSTEM_OFF`. On this machine
(`virt,secure=off`, no virtualization) QEMU expects PSCI calls through
`HVC`. The root task runs at EL0, where `HVC` is undefined as well. Only
seL4, at EL1, could issue it, and seL4 has no system call to do that.

## Decision

1. **The guest asks, the host acts.** On a halt, after its last message,
   the root task prints `%PAL-I-POWEROFF` on its own line and returns, as
   it would without a power-off. `poweroff()` loses its `HLT`.
2. **`run-qemu.sh` writes QEMU's PID** to a temporary file (`-pidfile`),
   one per QEMU, since the network test runs two at once, and
   drops `-semihosting-config`.
3. **`serial-filter.py` watches for the line.** It keeps scanning after the
   shim's banner, with the same carry-over it uses for a banner split
   across two reads. When it sees `%PAL-I-POWEROFF`, it sends SIGTERM to
   the PID in that file. QEMU takes SIGTERM as a clean shutdown.
4. **`--hvf` stays optional.** Nothing changes in how TCG runs; the halt
   just takes the same path under both.

## Alternatives considered

- **Keep semihosting and accept the hang under HVF.** No work, but
  `--hvf` would stay something you have to stop by hand, and every halt
  test would need TCG.
- **Semihosting under TCG, the filter under HVF.** Two ways to power off,
  and the filter is needed anyway. One path is simpler.
- **PSCI `SYSTEM_OFF` from the root task.** `HVC` and `SMC` are undefined
  at EL0; the root task would only fault.
- **seL4's SMC capability** (`KernelAllowSMCCalls`, `seL4_ARM_SMC_Call`).
  It makes the kernel issue an `SMC` for the holder of the capability. But
  QEMU expects PSCI through `HVC` on this machine, and an `SMC` it doesn't
  expect comes back to the guest as an undefined instruction. QEMU uses
  `SMC` only with `virtualization=on`. Under HVF that needs nested
  virtualization, which only newer Apple chips with recent macOS and QEMU
  have. It also changes the exception level Limine hands over at, which
  `pal/kernel/config.cmake` relies on, and the DTB seL4 dumped at configure
  time.
- **Patch seL4** with a system call that issues `HVC`. A fork of the kernel
  to maintain, for one call.
- **pvpanic-pci with `-action panic=shutdown`.** QEMU exits when the guest
  writes to the device. Guest-only, but the root task would have to find
  the device in PCI configuration space and map its BAR: far more code
  than the filter.

## Consequences

- QEMU powers off on a halt under both TCG and HVF.
- QEMU powers off only when it runs through `run-qemu.sh`, since the filter
  is what stops it. `cargo run -p boot`, `just boot` and `cargo test -p boot` all go through it.
  Started any other way, QEMU stays up after a halt, as it does under HVF
  today.
- The root task no longer needs semihosting from EL0, which let any EL0
  code reach the host's files through QEMU.
- The guest can't pass an exit status to the host. Nothing uses one yet. If
  a test needs it, the line can carry it, `%PAL-I-POWEROFF, status 1`, and
  the filter can exit with it.
- QEMU prints `terminating on signal 15` on stderr. If that gets in the
  way, the filter can send `quit` through a QMP socket instead.
- Follow-ups: implement the steps above, and update `docs/boot.md`, which
  says the root task powers QEMU off with semihosting. Making `--hvf` more
  than best effort (the CPU model, the counter frequency) is separate work.
