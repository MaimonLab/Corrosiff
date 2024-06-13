//! This module contains the implementation of the file data
//! information -- purely for I/O operations, does not know about imaging,
//! FLIM, etc.

mod file_format;
mod ifd;
mod tags;

pub use tags::{Tag, TiffTagID};
pub use ifd::{IFD, BigTiffIFD};
pub use file_format::FileFormat;