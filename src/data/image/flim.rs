pub mod histogram;
pub mod empirical_lifetime;

use bytemuck::try_cast_slice;
use binrw::io::{Read, Seek};
use ndarray::prelude::*;
use std::io::{
    Error as IOError,
    ErrorKind as IOErrorKind
};

use crate::{
    data::image::{
        dimensions::{
            macros::*,
            roll_inplace,
        },
        utils::{
        load_array_from_siff,
        photonwise_op,
    }},
    tiff::{Tag, TiffTagID::{Siff, StripByteCounts, StripOffsets}, IFD}, CorrosiffError,
};

use super::load_array_intensity;

#[binrw::parser(reader)]
fn _load_tau_d_raw<T : Into<u64>>(
    array : &mut ArrayViewMut3<u16>,
    strip_bytes : T,
    ydim : u32,
    xdim : u32
) -> binrw::BinResult<()>{
    let hdim = array.shape()[2];
    photonwise_op!(
        reader,
        strip_bytes,
        |photon : &u64| {
            array[
                [
                    photon_to_y!(photon, 0, ydim),
                    photon_to_x!(photon, 0, xdim),
                    photon_to_tau_USIZE!(photon) % hdim
                ]
            ] += 1;
        }
    );

    Ok(())
}

#[binrw::parser(reader)]
fn _load_tau_d_raw_registered<T : Into<u64>>(
    array : &mut ArrayViewMut3<u16>,
    strip_bytes : T,
    ydim : u32,
    xdim : u32,
    reg : (i32, i32)
) -> binrw::BinResult<()>{
    let hdim = array.shape()[2];
    photonwise_op!(
        reader,
        strip_bytes,
        |photon : &u64| {
            array[
                [
                    photon_to_y!(photon, reg.0, ydim),
                    photon_to_x!(photon, reg.1, xdim),
                    photon_to_tau_USIZE!(photon) % hdim
                ]
            ] += 1;
        }
    );

    Ok(())
}

#[binrw::parser(reader)]
fn _load_tau_d_compressed<T: Into<u64>>(
    array : &mut ArrayViewMut3<u16>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32
)-> binrw::BinResult<()>{
    reader.seek(std::io::SeekFrom::Current(
        -(ydim as i64 * xdim as i64 * std::mem::size_of::<u16>() as i64)
    ))?;

    let mut intensity_data : Vec<u8> = vec![0;
        ydim as usize * xdim as usize * std::mem::size_of::<u16>()
    ];
    reader.read_exact(&mut intensity_data)?;

    let intensity_data = try_cast_slice::<u8, u16>(&intensity_data)
    .map_err(|err| binrw::Error::Io(
        IOError::new(IOErrorKind::InvalidData, err)
    ))?;

    let mut arrival_data = vec![0; strip_byte_counts.into() as usize];
    reader.read_exact(&mut arrival_data)?;

    let arrival_data = try_cast_slice::<u8, u16>(&arrival_data)
    .map_err(|err| binrw::Error::Io(
        IOError::new(IOErrorKind::InvalidData, err)
    ))?;

    let mut arrival_time_pointer : usize = 0;

    let hdim = array.shape()[2];

    // Iterate over the intensity_array and `array` two-slowest axes together
    intensity_data.iter().enumerate().for_each(|(px_idx, intensity)| {
        arrival_data[arrival_time_pointer..arrival_time_pointer + *intensity as usize]
        .iter().for_each(|photon_arrival| {
            array[
                [
                    px_idx / xdim as usize,
                    px_idx % xdim as usize,
                    *photon_arrival as usize % hdim
                ]
            ] += 1;
        });
        arrival_time_pointer += *intensity as usize;
    });

    Ok(())
}

#[binrw::parser(reader, endian)]
fn _load_tau_d_compressed_registered<T : Into<u64>>(
    array : &mut ArrayViewMut3<u16>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    reg : (i32, i32)
)->binrw::BinResult<()>{

    _load_tau_d_compressed(
        reader,
        endian,
        (array, strip_byte_counts, ydim, xdim)
    )?;

    array.axis_iter_mut(Axis(2)).for_each(|mut image| roll_inplace(&mut image, (reg.0, reg.1)));

    Ok(())
}


/// Loads a single frame of a tau_d array with dimensions
/// `ydim` x `xdim` x `hdim` from a `.siff` file.
pub fn load_array_tau_d<I : IFD, ReaderT: Read + Seek>(
    reader : &mut ReaderT,
    ifd : &I,
    array_data : &mut ArrayViewMut3<u16>
) -> Result<(), CorrosiffError>{
    load_array_from_siff!(
        reader,
        ifd,
        (
            _load_tau_d_raw,
            (
                array_data,
                ifd.get_tag(StripByteCounts).unwrap().value(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32
            )
        ),
        (
            _load_tau_d_compressed,
            (
                array_data,
                ifd.get_tag(StripByteCounts).unwrap().value(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32
            )
        )
    )
}

/// Loads a single frame of a tau_d array with dimensions
/// `ydim` x `xdim` x `hdim` from a `.siff` file and applies
/// registration to the data.
pub fn load_array_tau_d_registered<I : IFD, ReaderT : Read + Seek>(
    reader : &mut ReaderT,
    ifd : &I,
    array_data : &mut ArrayViewMut3<u16>,
    reg : (i32, i32)
) -> Result<(), CorrosiffError>{
    load_array_from_siff!(
        reader,
        ifd,
        (
            _load_tau_d_raw_registered,
            (
                array_data,
                ifd.get_tag(StripByteCounts).unwrap().value(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                reg
            )
        ),
        (
            _load_tau_d_compressed_registered,
            (
                array_data,
                ifd.get_tag(StripByteCounts).unwrap().value(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                reg
            )
        )
    )

}