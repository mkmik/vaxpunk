+++
title = "In-browser demo (QEMU in WebAssembly)"
summary = "vaxpunk booting in a browser tab on QEMU compiled to WebAssembly"
aka = ["web demo", "just wasm-demo", "qemu-wasm", "tailgate", "web/", "GitHub Pages"]
kind = "tool"
intent = "goal"
research = "done"
refs = ["web/demo/index.html", "web/demo/lan.js", "web/qemu-wasm.sh", "web/mksite.sh", "web/demo-images.sh", "web/tailgate/main.go", "Justfile", ".github/workflows/boot.yml"]

[[feature]]
id = "boot"
title = "The system and data disks boot in the page, a terminal in xterm.js"
status = "done"
note = "ktock/qemu-wasm built in Docker; RESTART boots again with fresh disks"

[[feature]]
id = "lan"
title = "A LAN between the browser's tabs"
status = "done"
note = "PR #94; lan.js"

[[feature]]
id = "gateway"
title = "tailgate: a way out through a tailcat server, as a port forward or exit node"
status = "done"
note = "PR #96"

[[feature]]
id = "udp"
title = "UDP through the gateway, so DNS works"
status = "done"
note = "PR #124"

[[feature]]
id = "site"
title = "Published with the landing page and the API reference on GitHub Pages"
status = "done"
note = "mksite.sh; boot.yml"

[[needs]]
on = "/host/boot-runner"
why = "cargo run -p boot --images makes the ESP and disks it boots"

[[needs]]
on = "/host/apidoc"
why = "The site carries the API reference and its stylesheet"
soft = true

+++

Not a VMS piece: a way to try vaxpunk with nothing installed. The Pages
site serves QEMU compiled to WebAssembly (ktock/qemu-wasm, built with
Emscripten in Docker by `qemu-wasm.sh`), the EDK2 firmware and the
ESP and disk images the boot crate makes, gzipped, and boots them in the
page with the console in xterm.js. Log in as `SYSTEM`. The browser's
tabs share a LAN, so one vaxpunk can `SET HOST` to another, and
`tailgate`, a tailcat client in WebAssembly behind gVisor's TCP/IP
stack, gives the network a way out through a tailcat server. The data
disk lasts until the page reloads. `just wasm-demo` builds and serves it
locally.
