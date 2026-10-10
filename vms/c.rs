// C on the system disk (build.rs includes this): the C run-time library,
// crtl/, and SSL3, ssl3/, OpenSSL's API on Mbed TLS, whose release is
// downloaded once into the user's cache, as is the CA bundle SSL3 trusts. A C program links against both:
// crtl/decc$main.mar, its transfer address, then its own module, then
// SSL3$LIBSSL.OLB, SSL3's modules and Mbed TLS's, then DECC$CRTL.OLB.

/// The Mbed TLS release SSL3 is built on, and its SHA-256.
const MBEDTLS: &str = "mbedtls-4.1.1";
const MBEDTLS_SHA256: &str = "3359a349e23db3d5536fcee032ae7b2ecbfc08972fab643089b5cbf2a375c98c";

/// Mbed TLS's directories of sources, which SSL3 compiles all of; the
/// linker takes from the library only the modules a program needs.
const MBEDTLS_SOURCES: &[&str] = &[
    "library",
    "tf-psa-crypto/core",
    "tf-psa-crypto/extras",
    "tf-psa-crypto/utilities",
    "tf-psa-crypto/platform",
    "tf-psa-crypto/drivers/builtin/src",
];

/// vaxpunk's cache, which the Justfile's wasm-demo shares: the user's,
/// not the worktree's, so that each release is downloaded once.
fn cache() -> PathBuf {
    let home = PathBuf::from(env::var("HOME").expect("HOME"));
    let base = if cfg!(target_os = "macos") {
        home.join("Library/Caches")
    } else {
        env::var("XDG_CACHE_HOME").map_or(home.join(".cache"), PathBuf::from)
    };
    base.join("vaxpunk")
}

/// The file `name` in the cache, downloaded from `url` first if it isn't
/// there with the SHA-256 `sha256_hex`. A temporary name, then a rename, so
/// that two builds at once don't see half a file.
fn download(name: &str, url: &str, sha256_hex: &str) -> PathBuf {
    let cache = cache();
    fs::create_dir_all(&cache).unwrap();
    let file = cache.join(name);
    let checked = |path: &Path| {
        fs::read(path).is_ok_and(|bytes| {
            sha256(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                == sha256_hex
        })
    };
    if !checked(&file) {
        let part = cache.join(format!("{name}.{}", std::process::id()));
        let ok = Command::new("curl")
            .args(["-fsSL", "-o"])
            .arg(&part)
            .arg(url)
            .status()
            .is_ok_and(|s| s.success());
        assert!(ok, "can't download {url}");
        assert!(checked(&part), "{url} isn't {name}: its SHA-256 differs");
        fs::rename(&part, &file).unwrap();
    }
    file
}

/// Mbed TLS's release tree, unpacked in the cache from its tarball, which
/// download fetches. Unpacked under a temporary name, then renamed.
fn mbedtls() -> PathBuf {
    let cache = cache();
    let tree = cache.join(MBEDTLS);
    if tree.join("library").exists() {
        return tree;
    }
    let tarball = download(
        &format!("{MBEDTLS}.tar.bz2"),
        &format!(
            "https://github.com/Mbed-TLS/mbedtls/releases/download/{MBEDTLS}/{MBEDTLS}.tar.bz2"
        ),
        MBEDTLS_SHA256,
    );
    let unpack = cache.join(format!("{MBEDTLS}.{}", std::process::id()));
    fs::create_dir_all(&unpack).unwrap();
    let ok = Command::new("tar")
        .arg("xjf")
        .arg(&tarball)
        .arg("-C")
        .arg(&unpack)
        .status()
        .is_ok_and(|s| s.success());
    assert!(ok, "can't unpack {}", tarball.display());
    // Another build may have won the race; its tree is as good.
    let _ = fs::rename(unpack.join(MBEDTLS), &tree);
    let _ = fs::remove_dir_all(&unpack);
    tree
}

/// The CAs SSL3 trusts by default, SSL3$CERTS:CERT.PEM: Mozilla's, as curl
/// extracts them, the bundle Linux distributions ship. A newer one is a
/// new date and SHA-256 here (https://curl.se/docs/caextract.html).
fn ca_bundle() -> Vec<u8> {
    const DATE: &str = "2026-09-25";
    const SHA256: &str = "a41b5d356aea97a529fe27e0f7316d2f9d946d75927476cf9cf1b90637d00505";
    let name = format!("cacert-{DATE}.pem");
    fs::read(download(
        &name,
        &format!("https://curl.se/ca/{name}"),
        SHA256,
    ))
    .unwrap()
}

/// What every C module is compiled with, after velf's flags: the C
/// run-time library's headers instead of a hosted C library's, and gcc's
/// own freestanding ones.
fn c_flags() -> Vec<String> {
    let cross = env::var("CROSS_COMPILE").unwrap_or_else(|_| {
        let elf = Command::new("aarch64-elf-gcc").arg("--version").output();
        if elf.is_ok() {
            "aarch64-elf-"
        } else {
            "aarch64-linux-gnu-"
        }
        .into()
    });
    let gcc_include = Command::new(format!("{cross}gcc"))
        .arg("-print-file-name=include")
        .output()
        .unwrap_or_else(|e| panic!("{cross}gcc: {e}"));
    let gcc_include = String::from_utf8(gcc_include.stdout).unwrap();
    vec![
        "-nostdinc".into(),
        format!(
            "-I{}",
            Path::new("crtl/include").canonicalize().unwrap().display()
        ),
        "-isystem".into(),
        gcc_include.trim().into(),
    ]
}

/// Compiles a C source with velf's gcc and flags, and `args`, into an
/// object module: (file name, bytes). The ELF object goes to `object`.
fn compile_c(source: &Path, object: &Path, args: &[String]) -> (String, Vec<u8>) {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let elf = velf::gcc_with(source, object, &args)
        .unwrap_or_else(|e| panic!("{}: {e}", source.display()));
    convert(source, &elf)
}

/// compile_c, unless `dir` has the ELF object already. It gets there by
/// a rename, so that a build stopped halfway leaves none.
fn cached_c(source: &Path, dir: &Path, flags: &[String]) -> (String, Vec<u8>) {
    let object = dir.join(source.file_name().unwrap()).with_extension("o");
    if let Ok(elf) = fs::read(&object) {
        return convert(source, &elf);
    }
    let part = object.with_extension("part");
    let module = compile_c(source, &part, flags);
    fs::rename(&part, &object).unwrap();
    module
}

/// velf's object module of source's ELF object, named after the source,
/// cut to a module name's 31 characters.
fn convert(source: &Path, elf: &[u8]) -> (String, Vec<u8>) {
    let mut name = source.file_stem().unwrap().to_str().unwrap().to_uppercase();
    name.truncate(31);
    let records = velf::convert(elf, &name, vasm::Options::default().date)
        .unwrap_or_else(|e| panic!("velf {}: {e}", source.display()));
    (source.display().to_string(), vms_obj::obj::write(&records))
}

/// An object library of the modules: (file name, bytes).
fn library(name: &str, modules: &[(String, Vec<u8>)]) -> (String, Vec<u8>) {
    let mut lib = vlib::new(0);
    for (file, bytes) in modules {
        let warnings =
            vlib::replace(&mut lib, file, bytes, 0).unwrap_or_else(|e| panic!("vlib {name}: {e}"));
        assert!(warnings.is_empty(), "vlib {name}:\n{}", warnings.join("\n"));
    }
    (name.into(), lib.write())
}

/// The C run-time library, DECC$CRTL.OLB, and its transfer address module,
/// decc$main.mar's, which only C programs link.
fn crtl(out: &Path) -> ((String, Vec<u8>), (String, Vec<u8>)) {
    let dir = out.join("crtl");
    fs::create_dir_all(&dir).unwrap();
    let mut flags = c_flags();
    // The loops in memcpy and its kin would become calls to themselves.
    flags.push("-fno-tree-loop-distribute-patterns".into());
    let mut modules: Vec<_> = sources("crtl", &["c"])
        .iter()
        .map(|s| {
            compile_c(
                s,
                &dir.join(s.file_name().unwrap()).with_extension("o"),
                &flags,
            )
        })
        .collect();
    let mut macro32 = compile(&[
        PathBuf::from("crtl/decc$main.mar"),
        PathBuf::from("crtl/decc$vms.mar"),
    ]);
    let main = macro32.remove(0);
    modules.extend(macro32);
    modules.push(assemble(Path::new("crtl/decc$sys.m64")));
    (main, library("DECC$CRTL.OLB", &modules))
}

/// SSL3$LIBSSL.OLB: ssl3/'s modules and Mbed TLS's, configured by
/// ssl3/config. Mbed TLS's objects are kept in OUT_DIR under a hash of
/// what they are compiled with, the configuration, the C run-time
/// library's headers and the flags, so that they are compiled again only
/// when one changes; on many threads, as there are over a hundred.
fn ssl3(out: &Path) -> (String, Vec<u8>) {
    let tree = mbedtls();
    let mut flags = c_flags();
    flags.extend(
        [
            "include",
            "tf-psa-crypto/include",
            "tf-psa-crypto/drivers/builtin/include",
            "library",
            "tf-psa-crypto/core",
            "tf-psa-crypto/dispatch",
            "tf-psa-crypto/drivers/builtin/src",
            "tf-psa-crypto/utilities",
            "tf-psa-crypto/platform",
            "tf-psa-crypto/extras",
            "tf-psa-crypto/drivers/everest/include",
            "tf-psa-crypto/drivers/p256-m",
        ]
        .iter()
        .map(|d| format!("-I{}", tree.join(d).display())),
    );
    flags.extend([
        format!(
            "-I{}",
            Path::new("ssl3/config").canonicalize().unwrap().display()
        ),
        "-DMBEDTLS_USER_CONFIG_FILE=<tls_user.h>".into(),
        "-DTF_PSA_CRYPTO_USER_CONFIG_FILE=<crypto_user.h>".into(),
    ]);
    // Smaller code, for a system with 4 MB of memory; and no warnings,
    // which another gcc's would make errors in code that isn't ours.
    flags.extend(["-Os".into(), "-w".into()]);
    let headers = sources("crtl/include", &["h"]);
    let inputs: Vec<u8> = ["ssl3/config/tls_user.h", "ssl3/config/crypto_user.h"]
        .iter()
        .map(PathBuf::from)
        .chain(headers)
        .flat_map(|f| fs::read(f).unwrap())
        .chain(velf::GCC_FLAGS.join(" ").into_bytes())
        .chain(flags.join(" ").into_bytes())
        .collect();
    let hash: String = sha256(&inputs)[..6]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let dir = out.join(format!("{MBEDTLS}-{hash}"));
    fs::create_dir_all(&dir).unwrap();
    let srcs: Vec<PathBuf> = MBEDTLS_SOURCES
        .iter()
        .flat_map(|d| sources(tree.join(d).to_str().unwrap(), &["c"]))
        .collect();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let mut modules: Vec<(String, Vec<u8>)> = std::thread::scope(|scope| {
        let workers: Vec<_> = srcs
            .chunks(srcs.len().div_ceil(threads))
            .map(|chunk| {
                let (dir, flags) = (&dir, &flags);
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|s| cached_c(s, dir, flags))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().unwrap())
            .collect()
    });
    flags.pop();
    flags.push(format!(
        "-I{}",
        Path::new("ssl3/include").canonicalize().unwrap().display()
    ));
    let own = out.join("ssl3");
    fs::create_dir_all(&own).unwrap();
    for s in ["ssl3/ssl3$libssl.c", "ssl3/ssl3$libcrypto.c"] {
        let s = Path::new(s);
        modules.push(compile_c(
            s,
            &own.join(s.file_name().unwrap()).with_extension("o"),
            &flags,
        ));
    }
    library("SSL3$LIBSSL.OLB", &modules)
}
