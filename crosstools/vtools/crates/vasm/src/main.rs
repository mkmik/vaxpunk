//! vasm: assembles one source file into an object module.

fn main() -> std::process::ExitCode {
    vasm::main("vasm", None)
}
