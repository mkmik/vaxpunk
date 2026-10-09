# ADR-0031 — Random bytes come from VSI's $GET_ENTROPY, which the PAL serves from a virtio-rng device, and departures from VMS are marked in the sources

Oct 9, 2026 · @Marko Mikulicic

Proposed. `$GET_ENTROPY buffer, buflen` fills a buffer of at most 256
bytes with random bytes good enough for keys. It is VSI OpenVMS's
service, with VSI's name, arguments and limit. VAX and Alpha VMS had
nothing like it. Under it is a new PAL call, `GETENTROPY`, which reads
a virtio-rng device that QEMU fills from the host's random generator.
Both are marked in the sources as departures from VAX and Alpha VMS, and
the API reference shows that mark on their entries.

## Context

VAX and Alpha VMS had no source of random bytes in the kernel, and their
hardware had no random number generator. A program that needed a seed
made one from what the system shows: the time, and counters from
`$GETJPI`, `$GETRMI` and `$GETDVI`. OpenSSL's VMS code does exactly that
(`crypto/rand/rand_vms.c`). On a real machine those counters change with
disk timing, interrupts and users, so the seed is hard to guess, though
weak. VSI later added `SYS$GET_ENTROPY`, modelled on FreeBSD's
`getentropy()`. It gives at most 256 bytes a call, its status is
`SS$_NORMAL` or `SS$_RETRY`, and OpenSSL now uses it where it exists,
counting each byte as eight bits of entropy.

vaxpunk runs on QEMU, where the counters vary much less. Disk I/O is
polled and synchronous, the clock ticks every 10 ms, and every boot runs
the same startup in the same order. A seed made from the counters is
close to the same on every boot, and the time, the one value that
changes, can be guessed within seconds. That is fine for what VMS used
such values for: a password's salt only has to differ between users,
and `SET PASSWORD` and `AUTHORIZE` keep taking theirs from `$GETTIM`.
It is not fine for keys: a TLS client or WASMRUN's `random_get` (PRD-0007) would make keys an attacker could
guess.

The ARM64 random number instruction, `RNDR`, isn't there: QEMU's
`cortex-a57` doesn't have it, and not every Mac that runs vaxpunk under
HVF does either. QEMU's virtio-rng device works with every CPU, in the
browser demo too, and the PAL already drives virtio devices.

## Decision

1. **The service is VSI's `$GET_ENTROPY`.** `SYS$GET_ENTROPY buffer,
   buflen` and `$GET_ENTROPY_S BUFFER, BUFLEN` fill buflen bytes, at
   most 256, at buffer. The caller's mode must be able to write them.
   Its status is `SS$_NORMAL`, `SS$_BADPARAM` for more than 256,
   `SS$_ACCVIO` for a buffer the caller can't write, and
   `SS$_NOSUCHDEV` on a machine without the device. It never returns
   `SS$_RETRY`: the device doesn't run dry. It needs no privilege. Its
   code is `vms/exec/entropy.mar`, and it goes in the vector's
   *Security* group.
2. **The PAL reads a virtio-rng device, with `GETENTROPY`, call 0x49.**
   R0 is the buffer, R1 the byte count, and R0 comes back with the
   status. It works like `READLBLK`: the PAL finds the device among the
   virtio-mmio transports at boot, gives it a queue, and polls each
   request. The device may give fewer bytes than asked, so the PAL asks
   again until the buffer is full. `scripts/run-qemu.sh` and the web
   demo add `-device virtio-rng-device`, with QEMU's default backend,
   the host's random generator.
3. **No pool and no fallback.** The executive keeps no entropy pool and
   runs no generator of its own: QEMU's bytes come from the host's
   generator, which already is one. Without the device the service
   fails with `SS$_NOSUCHDEV`, instead of handing out bytes that look
   random and aren't. A program that wants VMS's old seed can still make
   one from `$GETTIM` and `$GETJPI`.
4. **Only keys use it.** What VMS made from the time stays that way:
   salts, and, when they come, DNS query IDs and DHCP transaction IDs.
   `$GET_ENTROPY` is for TLS, WASMRUN's `random_get` and anything else
   that makes keys.
5. **Departures from VMS are marked.** A paragraph of a comment block
   that begins `vaxpunk:` says where vaxpunk differs from VAX and Alpha
   VMS. apidoc shows it as a *Departs from VMS* note and puts a *Not VMS*
   tag on the entry. It does the same for every PAL call from 0x40,
   since Alpha PALcode has none there. `entropy.mar` has such a
   paragraph for the module and for `EXE$GET_ENTROPY`.

## Alternatives considered

- **VMS's way, a seed from the time and the counters.** No new code, but
  on QEMU the seed can be guessed, so any key made from it can be too.
- **A device, `RNA0:`, read with `$QIO`.** It would need a UCB, a driver
  and a channel for each reader, and VMS never had such a device. VSI
  chose a service.
- **A processor register, `MFPR #PR$_RNG`.** It is VAX-like, but a
  register read has no status, so a missing device can't be told apart
  from random bytes, and 256 bytes take 64 calls.
- **`RNDR`.** Not on the CPU QEMU emulates, and not on every host.
- **A generator in the executive, seeded once from the device.** More
  code in the executive for no gain while QEMU gives bytes this fast.
  It becomes worth it if a machine without a fast device ever matters.

## Consequences

- QEMU must have the device: `run-qemu.sh` and the web demo add it. A
  QEMU started without it boots, and only `$GET_ENTROPY` fails.
- The PAL prints `entropy: virtio-rng` at boot when it finds the device.
- `SVCTEST` checks that two calls give different bytes, and the errors.
- `GETENTROPY` waits for the device, as the disks' calls do, with the
  CPU busy. For 256 bytes from the host that is short.
- Other places where vaxpunk already differs from VAX and Alpha VMS,
  such as `$HASH_PASSWORD`'s `UAI$C_SHA256`, get a `vaxpunk:` paragraph
  as their comments are next changed.
