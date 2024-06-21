use ndarray::prelude::*;
use binrw::io::{Read, Seek};

use std::io::{
    Error as IOError,
    ErrorKind as IOErrorKind,
};

use crate::{
    tiff::{
        IFD,
        TiffTagID::{
            StripOffsets, 
            StripByteCounts,
            Siff,
        },
        Tag,
    },
    data::image::
    intensity::siff::{
        raw_siff_parser,
        compressed_siff_parser,
    },
};

/// A local struct for reading directly.
/// Only used internally for testing.
#[allow(dead_code)]
pub struct SiffFrame{
    pub intensity : ndarray::Array2<u16>,
}

impl SiffFrame {
    /// Parses a frame from a `.siff` file being viewed by
    /// `reader` using the metadata in the `ifd` argument
    /// to return a `SiffFrame` struct containing the intensity.
    /// 
    /// Does not move the `Seek` position of the reader because it
    /// is restored to its original position after reading the frame.
    /// 
    /// ## Arguments
    /// 
    /// * `ifd` - The IFD of the frame to load
    /// 
    /// * `reader` - The reader of the `.siff` file
    /// 
    /// ## Returns
    /// 
    /// * `Result<SiffFrame, IOError>` - A `SiffFrame` struct containing the intensity data
    /// for the requested frame.
    /// 
    /// ## Errors
    /// 
    /// * `IOError` - If the frame cannot be read for any reason
    /// this will throw an `IOError`
    #[allow(dead_code)]
    pub fn from_ifd<'a, 'b, I, ReaderT>(ifd : &'a I, reader : &'b mut ReaderT) 
    -> Result<Self, IOError> where I : IFD, ReaderT : Read + Seek {
        let cur_pos = reader.stream_position()?;

        reader.seek(
        std::io::SeekFrom::Start(
                ifd.get_tag(StripOffsets)
                .ok_or(
                    IOError::new(IOErrorKind::InvalidData, "Strip offset not found")
                )?.value().into()
            )
        ).or_else(|e| {reader.seek(std::io::SeekFrom::Start(cur_pos)).unwrap(); Err(e)})?;

        let parsed = match ifd.get_tag(Siff).unwrap().value().into() {
            0 => {
                raw_siff_parser(reader, binrw::Endian::Little,
                (
                    ifd.get_tag(StripByteCounts).unwrap().value(),
                    ifd.height().unwrap().into() as u32,
                    ifd.width().unwrap().into() as u32,
                )
            )},
            1 => {
                compressed_siff_parser(reader, binrw::Endian::Little, 
                (
                    ifd.height().unwrap().into() as u32,
                    ifd.width().unwrap().into() as u32,
                )
            )},
            _ => {Err(
                binrw::error::Error::Io(IOError::new(
                    IOErrorKind::InvalidData, "Invalid Siff tag")
                ))
            }
        }
        .map_err(|err| {
            reader.seek(std::io::SeekFrom::Start(cur_pos)).unwrap_or(0);
            IOError::new(IOErrorKind::InvalidData, err)
        })?;

        reader.seek(std::io::SeekFrom::Start(cur_pos)).unwrap_or(0);

        Ok(SiffFrame {
            intensity : parsed
        })
    }
}

/// Arbitrary dimensional siff intensity data
/// Not implemented for now...
#[allow(dead_code)]
pub struct SiffArray<D> {
    pub array : Array<u16, D>,
}