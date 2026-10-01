//! cargo run -p boot [-- [--gdb] [--hvf]]: copies the kernel, shim, root task
//! and boot volume cargo built into out/, stitches out/esp.img with mkesp.sh and becomes
//! scripts/run-qemu.sh, which gets the arguments.

use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, ExitCode};
use std::{env, fs, io};

fn main() -> ExitCode {
    if let Err(e) = boot() {
        eprintln!("boot: {e}");
    }
    ExitCode::FAILURE
}

/// Returns only on failure: on success the process is QEMU's.
fn boot() -> io::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let out = root.join("out");
    fs::create_dir_all(&out)?;
    for file in [kernel::ELF, shim::ELF, roottask::ELF, roottask::VOL].map(Path::new) {
        fs::copy(file, out.join(file.file_name().unwrap()))?;
    }
    let mkesp = Command::new(root.join("image/mkesp.sh")).status()?;
    if !mkesp.success() {
        return Err(io::Error::other("image/mkesp.sh failed"));
    }
    let qemu = Command::new(root.join("scripts/run-qemu.sh"))
        .args(env::args_os().skip(1))
        .exec();
    Err(qemu)
}
