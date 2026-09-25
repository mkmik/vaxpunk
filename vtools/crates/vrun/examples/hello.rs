//! Writes a hand-built image that prints "hello" and exits with success:
//!
//!     cargo run -p vrun --example hello -- hello.exe
//!     cargo run -p vrun -- hello.exe

use vms_obj::exe::{Eisd, Image, Section};

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "hello.exe".into());
    // adr x0, msg; mov x1, #6; svc #2 (put); mov x0, #1; ret; msg: "hello\n"
    let code = [
        0x100000a0u32,
        0xd28000c1,
        0xd4000041,
        0xd2800020,
        0xd65f03c0,
    ];
    let mut data: Vec<u8> = code.iter().flat_map(|i| i.to_le_bytes()).collect();
    data.extend(b"hello\n");
    let image = Image {
        name: "HELLO".into(),
        ident: "V1.0".into(),
        link_time: 0,
        transfer: 0x10000,
        sections: vec![Section {
            vaddr: 0x10000,
            size: data.len() as u32,
            flags: Eisd::M_EXE,
            data,
        }],
    };
    std::fs::write(&path, image.write()).unwrap();
}
