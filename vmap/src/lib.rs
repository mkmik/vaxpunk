//! Checks and queries the map of components in map/. See map/README.md for
//! the format and vmap/README.md for the commands.

pub mod compute;
pub mod json;
pub mod label;
pub mod map;
pub mod query;

pub use map::Map;
