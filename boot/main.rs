//! cargo run -p boot [-- [--autologin] [--gdb] [--hvf] [--uart1[=PORT]]
//! [--p1=VALUE ... --p8=VALUE]]: copies the kernel, shim, root task and system
//! disk cargo built into out/, stitches out/esp.img with mkesp.sh and becomes
//! scripts/run-qemu.sh, which gets the arguments: --p1 to --p8 are STARTUP_P1
//! to STARTUP_P8, for SYSTARTUP_VMS.COM. With --images it stops before QEMU,
//! for web/demo-images.sh.
//! --autologin writes [SYSEXE]SYSALF.DAT on out/sysdisk.img, so LOGINOUT logs
//! the console in as SYSTEM without asking.

use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, ExitCode};
use std::{env, fs, io};

use ods_image::{Conversion, Image, Mode};

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
    let mut args: Vec<_> = env::args_os().skip(1).collect();
    if let Some(i) = args.iter().position(|a| a == "--autologin") {
        args.remove(i);
        autologin(&out.join("sysdisk.img")).map_err(io::Error::other)?;
    }
    let mkesp = Command::new(root.join("boot/mkesp.sh")).status()?;
    if !mkesp.success() {
        return Err(io::Error::other("boot/mkesp.sh failed"));
    }
    if args == ["--images"] {
        return Ok(());
    }
    let qemu = Command::new(root.join("scripts/run-qemu.sh"))
        .args(args)
        .exec();
    Err(qemu)
}

/// The Automatic Login Facility's file, a line for the console: LOGINOUT
/// logs OPA0: in as SYSTEM.
fn autologin(disk: &Path) -> ods_image::Result<()> {
    let line = b"OPA0: SYSTEM\n";
    let mut vol = Image::open(disk, Mode::ReadWrite)?;
    let size = Some(line.len() as u64);
    vol.copy_in(
        &mut &line[..],
        "[SYSEXE]SYSALF.DAT",
        Conversion::LinesToRecords,
        size,
        None,
    )?;
    vol.flush()
}
