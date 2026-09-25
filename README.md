# vaxpunk

OpenVMS remake on arm64.

Milestone 0 boots an unmodified seL4 kernel on QEMU aarch64 through UEFI and
Limine, and runs a C root task that prints over the serial console.

## Host setup (once)

macOS (Apple Silicon, Homebrew) or Debian/Ubuntu (x86_64 or arm64):

```sh
scripts/setup-host.sh
```

It installs an aarch64 bare-metal C compiler, `cmake`, `ninja`, `dtc`, `uv`,
`mtools`, QEMU with its EDK2 firmware, and checks out the seL4 submodule.
