//! Records: each is declared once, as a list of fields, and its parser and
//! writer both come from that list.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// A format error found while parsing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The data ends before the structure does.
    Truncated,
    /// A field holds a value the format doesn't allow.
    Invalid(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated => f.write_str("truncated"),
            Error::Invalid(what) => write!(f, "invalid {what}"),
        }
    }
}

impl core::error::Error for Error {}

/// Reads fields from the front of a byte slice.
pub struct Reader<'a>(pub &'a [u8]);

impl<'a> Reader<'a> {
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        if self.0.len() < n {
            return Err(Error::Truncated);
        }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Ok(head)
    }
}

/// A field of a record: little-endian integers, byte arrays, counted strings
/// (`String`, a length byte then ASCII) and counted data (`Vec<u8>`, a
/// longword length then the bytes).
pub trait Field: Sized {
    /// Size in bytes; 0 for variable-length fields.
    const SIZE: usize;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
    fn put(&self, out: &mut Vec<u8>);
}

macro_rules! int_field {
    ($($t:ty),*) => {$(
        impl Field for $t {
            const SIZE: usize = size_of::<$t>();
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                Ok(<$t>::from_le_bytes(r.take(Self::SIZE)?.try_into().unwrap()))
            }
            fn put(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_le_bytes());
            }
        }
    )*};
}

int_field!(u8, u16, u32, u64);

impl<const N: usize> Field for [u8; N] {
    const SIZE: usize = N;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(r.take(N)?.try_into().unwrap())
    }
    fn put(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self);
    }
}

impl Field for String {
    const SIZE: usize = 0;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u8::read(r)?;
        let s = r.take(n.into())?;
        if !s.is_ascii() {
            return Err(Error::Invalid("name"));
        }
        Ok(s.iter().map(|&c| char::from(c)).collect())
    }
    fn put(&self, out: &mut Vec<u8>) {
        assert!(self.len() <= 255 && self.is_ascii(), "name: {self}");
        out.push(self.len() as u8);
        out.extend_from_slice(self.as_bytes());
    }
}

impl Field for Vec<u8> {
    const SIZE: usize = 0;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u32::read(r)?;
        Ok(r.take(n as usize)?.to_vec())
    }
    fn put(&self, out: &mut Vec<u8>) {
        (self.len() as u32).put(out);
        out.extend_from_slice(self);
    }
}

/// Declares a record struct with `SIZE` (of its fixed fields), `read`,
/// `parse` and `write`.
macro_rules! record {
    ($(#[$m:meta])* pub struct $name:ident { $($(#[$fm:meta])* pub $f:ident: $t:ty,)* }) => {
        $(#[$m])*
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name { $($(#[$fm])* pub $f: $t,)* }

        impl $name {
            /// Size in bytes of the fixed-size fields.
            pub const SIZE: usize = 0 $(+ <$t as $crate::record::Field>::SIZE)*;

            /// Reads the record from the front of `r`.
            pub fn read(r: &mut $crate::record::Reader<'_>) -> Result<Self, $crate::Error> {
                Ok(Self { $($f: $crate::record::Field::read(r)?,)* })
            }

            /// Reads the record from the start of `b`.
            pub fn parse(b: &[u8]) -> Result<Self, $crate::Error> {
                Self::read(&mut $crate::record::Reader(b))
            }

            /// Appends the record to `out`.
            pub fn write(&self, out: &mut alloc::vec::Vec<u8>) {
                $($crate::record::Field::put(&self.$f, out);)*
            }
        }
    };
}

pub(crate) use record;
