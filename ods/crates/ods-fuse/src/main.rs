//! ods-fuse: mounts a Files-11 disk image, read-only, with the mapping
//! proposed in docs/fuse.md (map.rs). Uses ods-image only.

mod fs;
mod map;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Mutex, mpsc};
use std::time::Duration;

use clap::Parser;
use fuser::{Config, MountOption};
use ods_image::{Image, Mode};

#[derive(Parser)]
#[command(name = "ods-fuse", about = "Mount a Files-11 (ODS-2/ODS-5) disk image, read-only", version)]
struct Args {
    image: PathBuf,
    mountpoint: PathBuf,
    /// List every version, older ones as name;N.
    #[arg(long)]
    versions: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let img = match Image::open(&args.image, Mode::ReadOnly) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let label = String::from_utf8_lossy(&img.home().volname()).trim().to_string();
    let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
    let fs = fs::Fs(Mutex::new(fs::Vfs::new(img, args.versions, uid, gid)));
    let mut config = Config::default();
    config.mount_options =
        vec![MountOption::RO, MountOption::FSName(format!("ods:{label}")), MountOption::Subtype("ods".into())];
    if cfg!(target_os = "macos") {
        config.mount_options.push(MountOption::CUSTOM("noappledouble".into()));
        config.mount_options.push(MountOption::CUSTOM(format!("volname={label}")));
    }
    // Signals go to a thread that waits for them, so Ctrl-C unmounts.
    let set = unsafe {
        let mut set: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut set);
        for sig in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
            libc::sigaddset(&mut set, sig);
        }
        libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut());
        set
    };
    let session = match fuser::spawn_mount(fs, &args.mountpoint, &config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("%ODS-F-MOUNT, cannot mount on {}: {e}", args.mountpoint.display());
            return ExitCode::FAILURE;
        }
    };
    eprintln!("{} mounted on {}; unmount with umount or Ctrl-C", args.image.display(), args.mountpoint.display());
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut sig = 0;
        unsafe { libc::sigwait(&set, &mut sig) };
        let _ = tx.send(());
    });
    loop {
        if rx.recv_timeout(Duration::from_millis(250)).is_ok() {
            let _ = session.umount_and_join();
            return ExitCode::SUCCESS;
        }
        if session.guard.is_finished() {
            return match session.join() {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("%ODS-F-FUSE, {e}");
                    ExitCode::FAILURE
                }
            };
        }
    }
}
