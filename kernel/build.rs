//! Builds seL4 through its own CMake into OUT_DIR, for the QEMU machine in
//! qemu.env. Reruns only when config.cmake, qemu.env, requirements.txt,
//! CROSS_COMPILE or the seL4 tree change. Dependents' build scripts get, as
//! DEP_SEL4_*: INCLUDE (libsel4's headers), PLATFORM (platform_gen.json) and
//! CROSS_COMPILE, the toolchain prefix the whole system builds with.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    for path in ["config.cmake", "qemu.env", "requirements.txt", "seL4"] {
        println!("cargo::rerun-if-changed={path}");
    }
    println!("cargo::rerun-if-env-changed=CROSS_COMPILE");
    assert!(
        Path::new("seL4/CMakeLists.txt").exists(),
        "kernel/seL4 is missing, run: git submodule update --init"
    );

    let cross = env::var("CROSS_COMPILE").unwrap_or_else(|_| {
        let elf = Command::new("aarch64-elf-gcc").arg("--version").output();
        if elf.is_ok() {
            "aarch64-elf-"
        } else {
            "aarch64-linux-gnu-"
        }
        .into()
    });
    let qemu = fs::read_to_string("qemu.env").unwrap();
    let qemu = |name: &str| {
        let value = qemu
            .lines()
            .find_map(|line| line.strip_prefix(name)?.strip_prefix('='));
        value
            .unwrap_or_else(|| panic!("qemu.env sets no {name}"))
            .to_owned()
    };
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let (venv, build, install) = (out.join("venv"), out.join("build"), out.join("install"));

    // The Python modules seL4's build scripts import.
    run(Command::new("uv")
        .args(["venv", "--allow-existing"])
        .arg(&venv));
    run(Command::new("uv")
        .args(["pip", "install", "--python"])
        .arg(&venv)
        .args(["-r", "requirements.txt"]));
    let path = env::var_os("PATH").unwrap();
    let path = [venv.join("bin")]
        .into_iter()
        .chain(env::split_paths(&path));
    let path = env::join_paths(path).unwrap();
    // No __pycache__ in seL4/: a new file there would trigger a rebuild.
    let tool = |name| {
        let mut cmd = Command::new(name);
        cmd.env("PATH", &path).env("PYTHONDONTWRITEBYTECODE", "1");
        cmd
    };

    // cmake -C only seeds a new cache, so configure from scratch.
    for dir in [&build, &install] {
        let _ = fs::remove_dir_all(dir);
    }
    let toolchain = env::current_dir().unwrap().join("seL4/gcc.cmake");
    run(tool("cmake")
        .args(["-G", "Ninja", "-S", "seL4", "-C", "config.cmake", "-B"])
        .arg(&build)
        .arg(format!("-DCMAKE_TOOLCHAIN_FILE={}", toolchain.display()))
        .arg(format!("-DCROSS_COMPILER_PREFIX={cross}"))
        .arg(format!("-DCMAKE_INSTALL_PREFIX={}", install.display()))
        .arg(format!("-DARM_CPU={}", qemu("QEMU_CPU")))
        .arg(format!("-DQEMU_MEMORY={}", qemu("QEMU_MEM")))
        .arg(format!("-DQEMU_GIC_VERSION={}", qemu("QEMU_GIC"))));
    run(tool("ninja").arg("-C").arg(&build).arg("install"));

    let include = install.join("libsel4/include");
    let platform = install.join("support/platform_gen.json");
    println!("cargo::metadata=include={}", include.display());
    println!("cargo::metadata=platform={}", platform.display());
    println!("cargo::metadata=cross_compile={cross}");
}

fn run(cmd: &mut Command) {
    let status = cmd.status().unwrap_or_else(|e| panic!("{cmd:?}: {e}"));
    assert!(status.success(), "{cmd:?} failed");
}
