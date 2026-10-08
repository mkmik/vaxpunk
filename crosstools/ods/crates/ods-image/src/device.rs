//! Block devices backed by image files.

use std::fs::File;
use std::io;
use std::os::unix::fs::FileExt;

use ods_core::{BLOCK, BlockDevice};

/// What wraps the blocks in the image file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Container {
    /// Blocks and nothing else: `.img`, `.dsk`, and the ODS-2 side of a
    /// dual-format ISO9660 CD, whose ISO structures live in blocks ODS-2
    /// leaves alone.
    Raw,
    /// A simh disk: raw blocks followed by a one-block footer naming the
    /// drive type.
    Simh { drive: String },
}

/// An image file, seen as consecutive 512-byte blocks.
pub struct FileDevice {
    file: File,
    blocks: u64,
}

impl FileDevice {
    /// Wraps an open image, recognizing its container.
    pub fn new(file: File) -> io::Result<(FileDevice, Container)> {
        let len = file.metadata()?.len();
        let (blocks, container) = match simh_footer(&file, len)? {
            Some((blocks, drive)) => (blocks, Container::Simh { drive }),
            None => (len / BLOCK as u64, Container::Raw),
        };
        Ok((FileDevice { file, blocks }, container))
    }

    /// A raw image of exactly `blocks` blocks, for a new volume.
    pub fn raw(file: File, blocks: u64) -> FileDevice {
        FileDevice { file, blocks }
    }

    pub fn file(&self) -> &File {
        &self.file
    }
}

/// Recognizes the footer open simh appends to disk images: the last block
/// starts with "simh", and holds the drive type and the big-endian sector
/// size and count. Returns the disk size in blocks and the drive type.
fn simh_footer(f: &File, len: u64) -> io::Result<Option<(u64, String)>> {
    if len < 2 * BLOCK as u64 || !len.is_multiple_of(BLOCK as u64) {
        return Ok(None);
    }
    let mut b = [0u8; BLOCK];
    f.read_exact_at(&mut b, len - BLOCK as u64)?;
    let be = |i: usize| u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as u64;
    let (sector, count) = (be(84), be(88));
    let bytes = sector * count;
    if &b[..4] != b"simh" || bytes == 0 || bytes % BLOCK as u64 != 0 || bytes > len - BLOCK as u64 {
        return Ok(None);
    }
    let drive = String::from_utf8_lossy(&b[68..84]).trim_end_matches('\0').to_string();
    Ok(Some((bytes / BLOCK as u64, drive)))
}

impl BlockDevice for FileDevice {
    type Error = io::Error;

    fn block_size(&self) -> usize {
        BLOCK
    }

    fn block_count(&self) -> u64 {
        self.blocks
    }

    fn read(&mut self, lbn: u64, buf: &mut [u8]) -> io::Result<()> {
        self.file.read_exact_at(buf, lbn * BLOCK as u64)
    }

    fn write(&mut self, lbn: u64, buf: &[u8]) -> io::Result<()> {
        self.file.write_all_at(buf, lbn * BLOCK as u64)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.sync_data()
    }
}
