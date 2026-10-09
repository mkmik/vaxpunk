//! The system disk, built by build.rs. No Rust code: the crate carries the build.

/// The system disk, a Files-11 ODS-2 volume: EXEC.EXE, the MACRO-32
/// executive the PAL starts, and the images of its processes in
/// [SYSEXE], SYSTEM's files in [SYSMGR]. QEMU attaches it as a read-only
/// virtio disk.
pub const DISK: &str = concat!(env!("OUT_DIR"), "/sysdisk.img");

#[cfg(test)]
mod uafhash {
    include!("uafhash.rs");

    #[test]
    fn sha256_known_answers() {
        let hex = |d: [u8; 32]| d.iter().map(|b| format!("{b:02x}")).collect::<String>();
        assert_eq!(
            hex(sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(sha256(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // Two blocks of padding, as $HASH_PASSWORD's messages have.
        assert_eq!(
            hash_password(b"MANAGER", 1, b"SYSTEM   "),
            hash_password(b"MANAGER", 1, b"SYSTEM")
        );
        assert_ne!(
            hash_password(b"MANAGER", 1, b"SYSTEM"),
            hash_password(b"MANAGER", 2, b"SYSTEM")
        );
    }
}
