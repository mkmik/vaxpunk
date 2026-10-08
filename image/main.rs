//! cargo run -p boot [-- [--gdb] [--hvf] [--uart1[=PORT]]]: copies the kernel,
//! shim, root task and system disk cargo built into out/, stitches out/esp.img
//! with mkesp.sh and becomes scripts/run-qemu.sh, which gets the arguments.
//! With --images it stops before QEMU, for web/demo-images.sh.

use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, ExitCode};
use std::{env, fs, io};

fn main() -> ExitCode {
    match boot() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("boot: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Returns on failure, or with --images; else the process becomes QEMU's.
fn boot() -> io::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let out = root.join("out");
    fs::create_dir_all(&out)?;
    for file in [kernel::ELF, shim::ELF, pal::ELF, vms::DISK].map(Path::new) {
        fs::copy(file, out.join(file.file_name().unwrap()))?;
    }
    let mkesp = Command::new(root.join("image/mkesp.sh")).status()?;
    if !mkesp.success() {
        return Err(io::Error::other("image/mkesp.sh failed"));
    }
    if env::args().skip(1).eq(["--images"]) {
        return Ok(());
    }
    let qemu = Command::new(root.join("scripts/run-qemu.sh"))
        .args(env::args_os().skip(1))
        .exec();
    Err(qemu)
}
