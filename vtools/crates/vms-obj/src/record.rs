//! Fixed-layout records: each is declared once, and its size, parser and
//! writer all come from that one field list.

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

/// A little-endian field of fixed size.
pub trait Field: Sized {
    const SIZE: usize;
    /// Reads the field from the start of `b`, which holds at least `SIZE` bytes.
    fn get(b: &[u8]) -> Self;
    fn put(&self, out: &mut Vec<u8>);
}

macro_rules! int_field {
    ($($t:ty),*) => {$(
        impl Field for $t {
            const SIZE: usize = size_of::<$t>();
            fn get(b: &[u8]) -> Self {
                <$t>::from_le_bytes(b[..Self::SIZE].try_into().unwrap())
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
    fn get(b: &[u8]) -> Self {
        b[..N].try_into().unwrap()
    }
    fn put(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self);
    }
}

/// Declares a record struct with `SIZE`, `parse` and `write`.
macro_rules! record {
    ($(#[$m:meta])* pub struct $name:ident { $($(#[$fm:meta])* pub $f:ident: $t:ty,)* }) => {
        $(#[$m])*
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name { $($(#[$fm])* pub $f: $t,)* }

        impl $name {
            /// Size in bytes.
            pub const SIZE: usize = 0 $(+ <$t as $crate::record::Field>::SIZE)*;

            /// Reads the record from the start of `b`.
            pub fn parse(b: &[u8]) -> Result<Self, $crate::Error> {
                if b.len() < Self::SIZE {
                    return Err($crate::Error::Truncated);
                }
                let mut _at = 0;
                Ok(Self {
                    $($f: {
                        let v = <$t as $crate::record::Field>::get(&b[_at..]);
                        _at += <$t as $crate::record::Field>::SIZE;
                        v
                    },)*
                })
            }

            /// Appends the record to `out`.
            pub fn write(&self, out: &mut alloc::vec::Vec<u8>) {
                $($crate::record::Field::put(&self.$f, out);)*
            }
        }
    };
}

pub(crate) use record;
