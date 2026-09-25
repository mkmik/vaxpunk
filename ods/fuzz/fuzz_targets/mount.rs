//! Mounts arbitrary bytes as a volume and walks everything it can reach.
//! Any panic or hang is a bug: bad data must come back as an error.
#![no_main]

use libfuzzer_sys::fuzz_target;
use ods_core::{BLOCK, BlockDevice, MFD, Volume};

struct Mem(Vec<u8>);

impl BlockDevice for Mem {
    type Error = ();
    fn block_size(&self) -> usize {
        BLOCK
    }
    fn block_count(&self) -> u64 {
        (self.0.len() / BLOCK) as u64
    }
    fn read(&mut self, lbn: u64, buf: &mut [u8]) -> Result<(), ()> {
        let at = (lbn as usize).checked_mul(BLOCK).ok_or(())?;
        buf.copy_from_slice(self.0.get(at..at + buf.len()).ok_or(())?);
        Ok(())
    }
    fn write(&mut self, _: u64, _: &[u8]) -> Result<(), ()> {
        Err(())
    }
    fn flush(&mut self) -> Result<(), ()> {
        Ok(())
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(mut v) = Volume::mount(Mem(data.to_vec()), false) else { return };
    let _ = v.free_blocks();
    let _ = v.files_in_use();
    let _ = v.verify();
    let mut todo = vec![MFD];
    let mut seen = Vec::new();
    while let Some(d) = todo.pop() {
        if seen.contains(&d) || seen.len() > 64 {
            continue;
        }
        seen.push(d);
        let Ok(entries) = v.list(d) else { continue };
        for e in entries.iter().take(64) {
            let Ok(info) = v.stat(e.fid) else { continue };
            let mut buf = vec![0u8; BLOCK * 4];
            let _ = v.read_blocks(e.fid, 1, &mut buf);
            if e.is_dir_name() && info.attrs.filechar & ods_core::fch::DIRECTORY != 0 {
                todo.push(e.fid);
            }
        }
    }
    let _ = v.lookup_path("[A.B]C.D;1");
});
