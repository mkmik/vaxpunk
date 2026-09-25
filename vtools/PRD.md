# PRD — ARM64 cross assembler, linker and QEMU runner for VMS object formats

Sep 25, 2026 · @Marko Mikulicic

## Context and goal

This is a sub-project of the effort to build an OpenVMS-style operating system on ARM64, on top of seL4. That OS will eventually need compilers — a MACRO-32 cross-compiler and a BLISS compiler are both planned — and those compilers need somewhere to put their output.

Starting with a compiler would mean writing a backend for a target that doesn't exist and producing code nothing can run. Using LLVM would mean inventing a new object format inside LLVM, for an OS/architecture combination upstream would never accept, and living with LLVM's build times. Neither is a good first step.

So this project goes **bottom-up**: the first tools are the ones every later tool depends on, and each produces something that runs.

1. an **object format** for ARM64 VMS, transliterated from the documented Alpha object language rather than invented;
2. a **cross assembler** that reads ARM64 assembly with VMS-style directives and writes that object format (OBJ);
3. a **cross linker** that combines OBJ modules into a VMS-style executable image (EXE);
4. a **runner** (monitor) that loads an EXE into a bare-metal QEMU virt machine, runs it in user mode (EL0) under a small boot stub with no OS, gives it a console, and exits cleanly when it returns.

The runner uses stock QEMU; nothing of our own interprets instructions. It only does what the future OS loader will do — map the image sections, set up a stack, transfer control, and provide a minimal way to print and exit — so that the object format and linker can be validated long before a kernel can load images.

All tools are written in Rust and run on the Mac. Later, the MACRO-32 and BLISS compilers target this assembler's object format (either directly or by emitting its assembly), and the real OS image loader reuses the same image-reading crate.

## Non-goals

- **No compiler.** No MACRO-32, BLISS or C front end here. Those are separate projects that consume this one.
- **No LLVM.** No LLVM backend, no LLVM object writer, no dependency on an LLVM build. LLVM tools (`llvm-mc`, `llvm-objdump`) may be used only as test oracles for instruction encoding.
- **No own instruction encoder if avoidable.** ARM64 encoding and decoding come from an existing Rust crate. The novel work is the object format, relocations, linking and conventions — not turning `add x0, x1, x2` into four bytes. Decoding does (`yaxpeax-arm`, in `vdump`). For encoding no crate fit; see *Encoding* below.
- **No emulator of our own.** The runner uses stock QEMU system mode. We write no interpreter and no instruction-level simulation.
- **No system services.** The runner provides a console and an exit, nothing resembling `SYS$` services, RMS, ASTs or condition handling.
- **No shareable images, no dynamic linking** in the first version. Single statically linked EXE only. The format keeps room for them.
- **No debug symbol format** (DST) in the first version beyond a link map. See open questions.

## Component overview

```
  foo.mar ──► vasm ──► foo.obj ─┐
  bar.mar ──► vasm ──► bar.obj ─┼──► vlink ──► prog.exe ──► vrun  (QEMU virt, bare metal)
                    lib.olb ───┘                  │
                   (vlib)                         └──► prog.map

        all tools share:  vms-obj  (OBJ/OLB/EXE read + write, no I/O assumptions)
        tools inspect:    vdump    (decoded dump of any OBJ, OLB or EXE)
```

(Tool names are placeholders.)

**`vms-obj`** — a library crate that defines the object module format, the object library format and the executable image format. Every record type is defined once, with both parse and serialize generated from the same definition. No file I/O, no host assumptions; it works on byte buffers. This crate is later reused by the OS image loader, so it must build `no_std` + `alloc`.

**`vasm`** — the cross assembler. ARM64 mnemonics, VMS-style directives and a macro facility; writes OBJ.

**`vlink`** — the cross linker. Reads OBJ and OLB, resolves symbols, applies relocations, writes EXE and a link map.

**`vlib`** — a minimal librarian that builds OLB object libraries. Small, but included early so the linker's library search is real from the start.

**`vdump`** — the equivalent of `ANALYZE/OBJECT` and `ANALYZE/IMAGE`: prints every record of a file, decoded, with the ARM64 instructions in text sections disassembled.

**`vrun`** — the runner. Loads an EXE into a bare-metal QEMU virt machine and runs it at EL0 behind a small boot stub; provides console output, exit status and fault reports.

## Workspace layout

```
vtools/                     (crates in the repository's Cargo workspace)
  crates/
    vms-obj/                no_std + alloc: OBJ, OLB, EXE formats
      src/obj/              module header, GSD, TIR, EOM records
      src/olb/              library header, index, modules
      src/exe/              image header, section descriptors, fixups
      src/reloc.rs          ARM64 relocation kinds and how to apply them
    vasm/                   assembler (binary + lib)
      src/lex.rs  parse.rs  macro.rs  directives.rs  encode.rs  emit.rs
    vlink/                  linker (binary + lib)
    vlib/                   librarian
    vdump/                  inspector
    vrun/                   runner: Rust host program + asm boot stub (stub/)
  docs/
    object-format.md        the ARM64 object language, as a delta from the Alpha spec
    image-format.md
    assembler.md            syntax and directive reference
    runner-abi.md           what an image sees when vrun starts it
  tests/
    asm/                    .mar sources + expected encodings
    run/                    .mar programs + expected console output and exit status
```

`docs/object-format.md` is a deliverable, not an afterthought: it is the only place the ARM64 VMS object format is defined, and later compilers are written against it.

## Object and image format

**Rule: transliterate, don't invent.** The starting point is the documented OpenVMS Alpha object language and image format (the Alpha `EOBJ` record family and the Alpha image header layout). Everything that isn't specific to the instruction set is kept as-is: record framing, record types, field layouts, symbol flags, program section attributes. Changes are made only where ARM64 forces them, and every change is listed in `docs/object-format.md` with the Alpha original beside it. First task: obtain and read the Alpha object language documentation and pin it in `docs/` by reference (title, order number, edition, URL, SHA-256). The manuals are HP/VSI copyright, so the repo holds our notes in our own words, never copies.

**Object module (OBJ).** A sequence of variable-length, typed records, as on Alpha:

- *module header* records — module name, version, creation time, language/tool name;
- *global symbol directory* (GSD) records — program section definitions with their attributes (alignment, `EXE`/`NOEXE`, `WRT`/`NOWRT`, `SHR`, `REL`/`ABS`, `CON`/`OVR`, `GBL`/`LCL`), global symbol definitions and references, procedure definitions pointing at their descriptors;
- *text, information and relocation* (TIR) records — the stack-machine command stream that stores data into program sections and expresses relocations (push a symbol or section base, add, store as quadword, etc.);
- *end of module* record — severity and optional transfer address.

**ARM64-specific changes.** Mostly in TIR:

- New store commands for ARM64 instruction relocations, each naming the instruction form and the bit field it patches: `B`/`BL` 26-bit PC-relative, conditional branch (`B.cond`, `CBZ`/`CBNZ`) and `LDR` (literal) 19-bit, `TBZ`/`TBNZ` 14-bit, `ADR` 21-bit, `ADRP` page-relative 21-bit, `ADD`/`LDR`/`STR` low-12 immediates (scaled by access size), and `MOVZ`/`MOVK` 16-bit chunks for absolute addresses. The instruction relocations in Arm's ELF ABI for AArch64 (AAELF64) are the completeness checklist.
- Alpha-specific commands (linkage-pair and `JSR`-hint optimisation commands, GP-relative stores) are dropped or replaced; the ARM64 equivalent of the linkage section is an open question (see below).
- Architecture code in the module header identifies ARM64.

**Procedure descriptors.** The format carries procedure descriptors as on Alpha, since the calling standard is to be a transliteration of the Alpha one. The exact ARM64 descriptor layout belongs to the calling standard, which is *not* designed in this project; the object format only needs a GSD procedure entry that points at a descriptor and a way to emit descriptor data. Keep the descriptor contents opaque here.

**Object library (OLB).** Library header, global symbol index mapping names to modules, and the modules themselves. Only what the linker needs to search it.

**Executable image (EXE).** Image header (with architecture code, transfer addresses, image name and ident), image section descriptors (virtual address, size, protection, whether demand-zero), and the section contents. Unused-for-now parts of the Alpha layout (shareable image lists, fixup sections for shared images, symbol vectors) keep their slots so later versions don't reshape the header.

**Page size.** Alpha used 8 KB pages; ARM64 hosts use 4 KB or 16 KB. Image sections are aligned to **64 KB** so the same EXE loads on any ARM64 page size (and on the eventual seL4 target). Revisit once the OS's page size is chosen.

**Byte order.** Little-endian throughout, as on Alpha.

## Assembler requirements (`vasm`)

**Instructions: standard ARM64.** Mnemonics, register names, operand syntax and addressing modes follow the standard ARM64 assembly syntax as accepted by LLVM and GNU `as`. No invented mnemonics. Anyone who can read ARM64 assembly can read this.

**Encoding: small tables, checked against GNU `as`.** No Rust crate turns ARM64 text into instructions correctly without LLVM. `asm-rs` silently mis-encodes some forms (`ldr q0, [x1, #16]` comes out as `ldr w0`, `orr w0, w1, #0x80000000` gets the wrong immediate) and has no floating point. `aarchmrs-instructions`, generated from Arm's machine-readable spec, only packs fields, with one function per mnemonic, width and form: the assembler would still parse operands and pick every form itself, and wire about 250 functions by name, for opcode constants the oracle checks anyway. `dynasm-rs` works only at Rust compile time; Keystone is LLVM. So `encode.rs` packs fields from one template per instruction class, and every supported form is checked bit for bit against GNU `as` (`tests/encode.rs`). For an instruction that refers to a symbol the linker resolves, the assembler encodes a zero field and records the relocation. `yaxpeax-arm` decodes, for `vdump`.

**Directives: VMS style.** This is where the assembler diverges from the ARM64 norm, deliberately. Directives are modelled on MACRO-64, DEC's native Alpha assembler, and MACRO-32, adapted to ARM64:

- program sections: `.PSECT name, attr, attr, …` with the VMS attribute set, instead of `.text`/`.data`/`.section`;
- symbols: `.GLOBAL`/`.EXTERNAL`/`.WEAK`, `=` and `==` for local and global assignments;
- data: `.BYTE .WORD .LONG .QUAD .ADDRESS .ASCII .ASCIZ .ASCIC .ASCID` (`.ASCID` emits a string *descriptor*, as in MACRO), `.BLKB .BLKW .BLKL .BLKQ`, `.ALIGN`;
- procedures: a directive family equivalent to MACRO-64's procedure-descriptor support, emitting the GSD procedure entry and descriptor data — its exact contents parameterised until the calling standard is designed;
- module: `.TITLE`, `.IDENT`, `.END [transfer]`.

**Macros.** A real macro facility in the MACRO tradition: `.MACRO`/`.ENDM` with keyword and default arguments, `.IF`/`.IIF` with the usual conditions, `.IRP`/`.IRPC`/`.REPEAT`, local labels, and `.LIBRARY` for macro libraries. This is what makes an assembler pleasant to write a kernel in, and what will carry system-service call macros later.

**Case and names.** Symbols are case-insensitive and folded to upper case in the object file, as on VMS. Names allow `$` and `_`.

**Output.** One OBJ per source file, plus an optional listing file (source, addresses, encoded bytes, macro expansions) — the classic `/LIST` output.

**Diagnostics.** Errors point at file, line and column, and show macro expansion context.

## Linker requirements (`vlink`)

- Inputs: OBJ files and OLB libraries, in the order given; libraries searched only for unresolved symbols, as on VMS.
- Collects program sections by name and attribute across modules, concatenating `CON` sections and overlaying `OVR` ones, honouring alignment.
- Groups sections into image sections by protection (read-only code, read-only data, writable data, demand-zero).
- Resolves global symbols; reports undefined and multiply-defined symbols with the modules involved.
- Executes each module's TIR command stream against the final addresses, applying every ARM64 relocation with range checks — a `B` target out of ±128 MB or an `ADRP` target out of ±4 GB is a link error, not silent truncation. (No range-extension veneers in the first version; errors are fine.)
- Chooses a transfer address from the first module whose end-of-module record names one, or from a command-line option.
- Places the image at a base address given on the command line (default fixed), producing a non-relocatable image for now. Position-independent images are an open question.
- Writes the EXE and, on request, a map file in the style of VMS `LINK/MAP`: section list, symbol table by name and by address, module contributions.
- Command line is DCL-flavoured but host-friendly, e.g. `vlink /EXE=prog.exe /MAP=prog.map a.obj b.obj lib.olb/LIBRARY`, with a plain Unix-style alternative.

## The runner (`vrun`)

`vrun` runs an EXE inside QEMU system mode (`qemu-system-aarch64 -M virt`), with no OS. A small boot stub runs at EL1 and the image runs in user mode (EL0), as it will on seL4, where only the kernel runs at EL1. The image gets the lower half of the guest address space, minus a small stub region, so it loads at the addresses the linker chose. The host OS never decides placement.

**Why not native on the host.** The first design mapped the image straight into a macOS process. That fails: on Apple Silicon nothing can be mapped below 4 GB, so an image linked at the VMS default of 0x10000 can never load, and shrinking `__PAGEZERO` gets the process killed at exec. Above 4 GB, which ranges are free is up to the host (on macOS 15.7: 16–60 GB and from 448 GB up, with the shared cache and a reserved range in between). Letting host quirks shape the image layout is the wrong trade, so the runner moved into a VM.

**Shape.** Two pieces:

- `vrun` — a small host program in Rust. It reads the EXE with `vms-obj`, plans the load, builds the page tables, starts QEMU, relays the serial console, and turns the stub's status and fault lines into an exit code and symbols. No `unsafe`.
- the **boot stub** — a short freestanding ARM64 assembly file, the only code that runs privileged. It turns on the MMU with vrun's page tables, installs exception vectors, enters the image at EL0, and serves its monitor calls. Written in GNU syntax and built with the cross binutils the repo already uses for the shim (`aarch64-elf-as`); rebuilt with `vasm`/`vlink` once they can handle it.

**Loading.**

1. Read the EXE, check the architecture code and header.
2. Lay out guest RAM: the stub, the page tables, the runner info block, the image stack and the image sections, each section 64 KB aligned. RAM starts at 0x4000\_0000 on `virt`, but without `-kernel` QEMU puts its device tree in the first 1 MB, so the layout starts at 0x4020\_0000.
3. Build the page tables (4 KB granule, lower half, `TTBR0`): the image sections at their **link virtual addresses**, accessible from EL0 with their protection (read-only, read-write, read-execute; never writable and executable at once); the image stack with an unmapped guard page below it; the return page (see below); and the stub region and UART page mapped EL1-only at their physical addresses. vrun rejects an image that overlaps the stub region or the UART page.
4. Write it all as one RAM image and start QEMU with `-device loader,file=…,addr=0x40200000,force-raw=on` plus `-device loader,addr=<stub entry>,cpu-num=0` to set the start PC. No ELF anywhere; QEMU just copies bytes to addresses (`force-raw` stops it from sniffing the file for ELF or uImage headers).

Because the MMU is on, virtual addresses are free: an image linked at 0x10000 runs exactly as linked. The upper half (`TTBR1`) stays empty for now and is the natural home for VMS system space later.

**Transfer of control.** QEMU starts the stub at EL1 (`virt` default, no EL2/EL3), interrupts masked. The stub loads `MAIR_EL1`, `TCR_EL1` and `TTBR0_EL1`, turns on the MMU and caches, sets `VBAR_EL1`, enables FP/SIMD (`CPACR_EL1.FPEN`, whose reset value isn't guaranteed), and enters the image with `eret`: `SPSR_EL1` selects EL0, `ELR_EL1` holds the transfer address, `SP_EL0` the image stack, and the entry registers come from `docs/runner-abi.md`. Exceptions from EL0 switch to the stub's own stack, so even a stack overflow into the guard page gets a clean report.

**Entry ABI (provisional).** `x0` = pointer to the *runner info block* (argument string as a descriptor, runner version, flags); `lr` = the return page; `sp` = image stack. Marked provisional; replaced by the calling standard once it exists. No reserved registers: `x18` is free, unlike on macOS.

**Return page.** An image at EL0 can't return into the stub, so `lr` points at an EL0-executable page holding a single exit call. Returning from the entry point therefore exits with `x0` as the status, as on VMS, where the image returns to the image activator and that calls `$EXIT`.

**Monitor calls and console.** `SVC #imm` from the image traps to the stub, which offers a few services: put a string, dump registers, exit with status. Images will use the same instruction for seL4 system calls, and `BRK` stays free for debuggers. The console is the PL011 UART on `virt`, which only the stub touches; the image has no device memory mapped. QEMU sends the UART to vrun's stdout (serial on stdio).

**Exit.** The exit service, or returning from the entry point, ends the run. The stub prints one machine-readable status line, then powers off with PSCI `SYSTEM_OFF` (`HVC #0`), which works under TCG and HVF alike. No semihosting: under HVF its `HLT #0xF000` is an undefined instruction in the guest, and QEMU's exit code would carry only 8 bits of the status anyway. VMS convention: low bit set means success; vrun maps the status line to host exit code 0 for success, 1–255 otherwise, and prints the full 32-bit status with `--verbose`.

**Faults.** The stub's vectors catch data aborts, instruction aborts, and undefined or privileged instructions from the image, print one machine-readable line with `ESR`, `ELR` and `FAR`, and power off. vrun spots that line in the serial stream and prints a `%VRUN-F-ACCVIO`-style message with the PC translated to *section + offset* and, from the link map, the nearest symbol. `--timeout` kills a hung guest.

**Debugging.**

- `--gdb` starts QEMU with `-s -S` (halted, GDB stub on port 1234) and prints the attach command for `lldb` (`gdb-remote 1234`) or `gdb-multiarch`. Stepping works from the stub's first instruction.
- `--map prog.map` loads the linker's map for symbolised messages.
- Stretch goal: `--sym out.elf` writes a symbol-only ELF from the link map, loaded into the debugger so it shows real names. A debugging aid only, never a project format.

**Speed and hosts.** Default accelerator is TCG: deterministic, and runs anywhere QEMU does, including x86 Linux CI. `--hvf` uses hardware virtualisation on Apple Silicon. vrun passes `-cpu max` under TCG and `-cpu host` under HVF; `virt`'s default CPU is 32-bit. Startup is not a concern: a probe stub ran start to finish in about 0.04 s. Same QEMU and `virt` machine as the seL4 boot skeleton, so scripts and debugging habits carry over.

## Testing strategy

**Encoding against an oracle.** For every instruction form the assembler supports, a test assembles it with `vasm` and with GNU `as` from the cross binutils the repo already installs (`aarch64-elf-as`; `llvm-mc` also works) and compares the four bytes. Generated tables cover each form with a spread of registers and immediates. The oracle is only a test tool.

**Format round trip.** Every OBJ, OLB and EXE record: parse → serialize → identical bytes. Run over everything the test suite produces.

**Relocation tests.** For each relocation kind: a two-module program where the reference sits in one module and the target in another, linked at several bases, including targets just inside and just outside the instruction's range (the latter must fail to link with a clear message). Checked both by decoding the patched instruction and by running it.

**Execution tests.** `tests/run/` holds small assembly programs with expected console output and exit status: hello world, arithmetic, loops, calls across modules, data in writable sections, demand-zero sections, a library-resolved routine, an explicit exit call, and expected fault reports for an access violation, a privileged instruction and a stack overflow into the guard page. These run under `vrun` in CI on any host with QEMU (TCG), ARM64 or x86. TCG doesn't model caches and forgives missing barriers, so changes to the stub are also run under `--hvf` on a Mac.

**Inspector as test aid.** `vdump` output for a set of reference programs is checked in and compared, so format changes are always visible in review.

**Fuzzing.** `cargo fuzz` targets for the OBJ/OLB/EXE parsers and for the assembler's lexer and parser: no panics, no hangs.

## Open questions

- **Spec source.** Where to get the Alpha object language and image format documentation (OpenVMS linker manual appendices, `EOBJDEF`/`EIHDDEF` definitions from published headers). GNU binutils implements the format and is a working reference: `bfd/vms-alpha.c` (objects and images), `bfd/vms-lib.c` (libraries), `include/vms/*.h` (record layouts). It is GPL-3, so the FreeVMS rule applies: learn the layouts, never copy code. Collect before writing `vms-obj`.
- **Linkage sections.** Alpha code reaches data and other routines through a per-procedure linkage section pointed to by the procedure descriptor. ARM64 has `ADRP`+`ADD` PC-relative addressing that makes this less necessary. Keep linkage sections for fidelity with the Alpha calling standard, or drop them? Decide together with the calling standard; the object format should support both.
- **Assembler name and source suffix.** `.MAR` suggests MACRO-32, which this is not. Pick a name (and suffix) for the ARM64 MACRO-style assembler.
- **Position-independent images.** Needed later for shareable images and possibly for address-space randomisation. The first version links at a fixed base.
- **Debug symbol table.** Emit VMS-style DST records (so a future VMS-native debugger can use them) or just the link map for now. Leaning map only.
- **Page size and image base** for the eventual seL4 target — affects section alignment and default base. 64 KB alignment chosen as the safe choice meanwhile.
- **Relation to the MACRO-32 cross-compiler.** Does it emit this assembler's source text (simple, debuggable) or call `vms-obj` directly (faster, no second parse)? Probably source text first.

## Work order

Each step ends with something you can run or look at.

1. **Spec notes.** Pin the Alpha object and image format documentation in `docs/` by reference; write the first draft of `object-format.md` as a delta. *Visible:* the document.
2. **Hand-built image and the runner.** `vms-obj` image writer only; a test builds a tiny EXE from hand-encoded bytes (`mov x0, #1; ret`); `vrun` loads it into QEMU behind the boot stub (MMU on, image at its link address, at EL0) and runs it; the return page, status line and PSCI power-off carry the status out. *Visible:* `vrun tiny.exe` exits 0.
3. **Console.** Runner info block and the `SVC` monitor calls (put a string, dump registers, exit). *Visible:* hand-built image prints "hello".
4. **Object format and `vdump`.** OBJ read/write with round-trip tests; `vdump` for OBJ and EXE. *Visible:* decoded dump of a hand-built OBJ.
5. **Minimal assembler.** Instructions, labels, `.PSECT`, data directives, `.END`; one module. *Visible:* `hello.mar` → OBJ, inspected with `vdump`.
6. **Minimal linker.** One module, one image, fixed base, map file. *Visible:* `vasm hello.mar && vlink hello.obj && vrun hello.exe` prints "hello" — **first end-to-end milestone**.
7. **Multiple modules and relocations.** All ARM64 relocation kinds, global symbols, range errors. *Visible:* the relocation test suite passes under `vrun`.
8. **Macros.** Full macro facility and macro libraries. *Visible:* a console-output macro library used by the test programs.
9. **Libraries.** `vlib` and linker library search. *Visible:* a program linking a routine pulled from an OLB.
10. **Faults and debugging.** Symbolised fault reports, `--gdb`, map-driven symbols. *Visible:* an access violation reported as symbol + offset; LLDB attached and stepping through image code.
11. **Procedure descriptors.** Once the calling standard exists: descriptor directives, GSD procedure entries, and entry through the real calling standard. *Visible:* a call across modules going through descriptors.
