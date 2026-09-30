//! vmacro: compiles one MACRO-32 source file into an object module.

fn main() -> std::process::ExitCode {
    vasm::main("vmacro", Some(&mut vmacro::Macro32::default()))
}
