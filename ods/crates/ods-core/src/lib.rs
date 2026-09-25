//! Files-11 On-Disk Structure, levels 2 and 5, with no host dependencies.
//!
//! The core reads and writes numbered 512-byte blocks through
//! [`BlockDevice`] and knows nothing else about its environment: no files,
//! no clock (the caller passes one), no allocator beyond `alloc`.
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod bitmap;
mod dir;
mod file;
mod index;
mod init;
pub mod layout;
pub mod name;
mod ops;
mod verify;
mod volume;

use core::fmt;

pub use dir::{DirEntry, MFD};
pub use file::{Alloc, Attributes, FileInfo};
pub use init::{InitParams, initialize};
pub use layout::{BLOCK, Fid, RecordAttrs};
pub use name::{FileName, Spec, Version};
pub use ops::NewFile;
pub use verify::{Finding, Report, Severity};
pub use volume::{Level, Volume};

/// Storage the core runs on: an array of blocks.
///
/// `buf` holds one or more whole blocks; a multi-block transfer covers
/// consecutive LBNs. The core issues multi-block transfers only for file
/// data and never depends on them being atomic.
pub trait BlockDevice {
    type Error: fmt::Debug;
    /// Always 512 for Files-11; checked at mount.
    fn block_size(&self) -> usize;
    fn block_count(&self) -> u64;
    fn read(&mut self, lbn: u64, buf: &mut [u8]) -> core::result::Result<(), Self::Error>;
    fn write(&mut self, lbn: u64, buf: &[u8]) -> core::result::Result<(), Self::Error>;
    fn flush(&mut self) -> core::result::Result<(), Self::Error>;
}

/// Everything that can go wrong. `Corrupt` is the only answer to malformed
/// on-disk data: the core never panics on it.
#[derive(Debug)]
pub enum Error<E> {
    Device(E),
    /// No valid home block on the volume.
    NoHomeBlock,
    /// A structure failed validation.
    Corrupt {
        what: &'static str,
        lbn: u64,
    },
    /// No such file (or no such version).
    NotFound,
    /// A directory in the path does not exist.
    DirNotFound,
    /// The file ID's sequence number does not match the header: the file
    /// it named was deleted.
    Stale(Fid),
    BadName(&'static str),
    Exists,
    NotDirectory,
    DirNotEmpty,
    /// The operation needs an explicit version (delete).
    NoVersion,
    /// A version above 32767 would be needed.
    VersionOverflow,
    DeviceFull,
    /// The index file already holds the maximum number of files.
    HeaderFull,
    ReadOnly,
    /// A reserved file (INDEXF.SYS and friends) cannot be deleted or moved.
    Reserved,
    /// Access past the end of the file's allocation.
    BeyondEof,
    Unsupported(&'static str),
    Invalid(&'static str),
}

impl<E> From<E> for Error<E> {
    fn from(e: E) -> Self {
        Error::Device(e)
    }
}

impl<E: fmt::Debug> fmt::Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Device(e) => write!(f, "device error: {e:?}"),
            Error::NoHomeBlock => f.write_str("no valid home block"),
            Error::Corrupt { what, lbn } => write!(f, "{what} at LBN {lbn}"),
            Error::NotFound => f.write_str("file not found"),
            Error::DirNotFound => f.write_str("directory not found"),
            Error::Stale(fid) => write!(f, "file ID {fid} is stale"),
            Error::BadName(why) => write!(f, "bad file name: {why}"),
            Error::Exists => f.write_str("file already exists"),
            Error::NotDirectory => f.write_str("not a directory"),
            Error::DirNotEmpty => f.write_str("directory not empty"),
            Error::NoVersion => f.write_str("version number required"),
            Error::VersionOverflow => f.write_str("version number above 32767"),
            Error::DeviceFull => f.write_str("device full"),
            Error::HeaderFull => f.write_str("index file full"),
            Error::ReadOnly => f.write_str("volume is read-only"),
            Error::Reserved => f.write_str("reserved file"),
            Error::BeyondEof => f.write_str("beyond end of file"),
            Error::Unsupported(what) => write!(f, "unsupported: {what}"),
            Error::Invalid(what) => write!(f, "invalid: {what}"),
        }
    }
}

pub type Result<T, E> = core::result::Result<T, Error<E>>;

/// File characteristics (FH2$L_FILECHAR bits).
pub mod fch {
    pub const WASCONTIG: u32 = 1 << 0;
    pub const NOBACKUP: u32 = 1 << 1;
    pub const WRITEBACK: u32 = 1 << 2;
    pub const READCHECK: u32 = 1 << 3;
    pub const WRITCHECK: u32 = 1 << 4;
    pub const CONTIGB: u32 = 1 << 5;
    pub const LOCKED: u32 = 1 << 6;
    pub const CONTIG: u32 = 1 << 7;
    pub const BADACL: u32 = 1 << 11;
    pub const SPOOL: u32 = 1 << 12;
    pub const DIRECTORY: u32 = 1 << 13;
    pub const BADBLOCK: u32 = 1 << 14;
    pub const MARKDEL: u32 = 1 << 15;
    pub const NOCHARGE: u32 = 1 << 16;
    pub const ERASE: u32 = 1 << 17;
}

/// Record formats (low nibble of `RecordAttrs::rtype`).
pub mod rfm {
    pub const UDF: u8 = 0;
    pub const FIX: u8 = 1;
    pub const VAR: u8 = 2;
    pub const VFC: u8 = 3;
    pub const STM: u8 = 4;
    pub const STMLF: u8 = 5;
    pub const STMCR: u8 = 6;
}

/// Record attributes (`RecordAttrs::rattrib` bits).
pub mod rat {
    pub const FTN: u8 = 1 << 0;
    pub const CR: u8 = 1 << 1;
    pub const PRN: u8 = 1 << 2;
    pub const BLK: u8 = 1 << 3;
    pub const MSBRCW: u8 = 1 << 4;
}
