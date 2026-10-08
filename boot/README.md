# boot: putting it together and booting it

The `boot` crate puts the pieces together and runs them in QEMU.

- `main.rs`: `cargo run -p boot` copies `kernel.elf`, `shim.elf`,
  `roottask.elf` (from [pal/](../pal)) and `sysdisk.img` (from
  [vms/](../vms)) to `out/`, stitches `out/esp.img` with `mkesp.sh`, and
  becomes `scripts/run-qemu.sh`. `--images` stops before QEMU.
- `mkesp.sh`: makes the FAT32 ESP with Limine, `limine.conf` and the three
  ELFs, using mtools only.
- `limine.conf`: tells Limine to load the shim, with seL4 and the root task
  as modules.
- `tests/`: boot the whole system and drive its console. `boot.rs` covers
  the STARTUP checks and a DCL session, `network.rs` the TCP/IP component,
  and `rms.rs` RMS on relative and indexed files, checked against what
  real OpenVMS printed for the same files.
  Run them with `cargo test --release -p boot`; the console is in
  `out/serial.log`.
