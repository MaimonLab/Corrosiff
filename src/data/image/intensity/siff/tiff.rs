use binrw;
use ndarray::prelude::*;
use bytemuck::try_cast_slice;
use std::io::{
    Error as IOError,
    ErrorKind as IOErrorKind,
};

use crate::data::image::dimensions::roll_inplace;

/// Parses a `tiff` format frame and fills an array
/// with the data
/// 
/// Expected to be at the data strip already.
#[binrw::parser(reader)]
pub fn load_array_tiff(
        array : &mut ArrayViewMut2<u16>,
        ydim : u32,
        xdim : u32
    ) -> binrw::BinResult<()> {
    
    let mut data : Vec<u8> = vec![0; 
        ydim as usize * xdim as usize * std::mem::size_of::<u16>()
    ];
    reader.read_exact(&mut data)?;

    let data = try_cast_slice::<u8, u16>(&data).map_err(|err| binrw::Error::Io(
        IOError::new(IOErrorKind::InvalidData, err))
    )?;

    array.iter_mut().zip(data.iter()).for_each(|(a, &v)| *a = v);

    Ok(())
}

/// Parses a `tiff` format frame and fills an array
/// with the data, then applies registration
/// 
/// Expected to be at the data strip already.
#[binrw::parser(reader, endian)]
pub fn load_array_tiff_registered(
        array : &mut ArrayViewMut2<u16>,
        ydim : u32,
        xdim : u32,
        registration : (i32, i32)
    ) -> binrw::BinResult<()> {
        
    load_array_tiff(
        reader,
        endian,
        (
            array,
            ydim,
            xdim
        )
    )?;

    roll_inplace(array, registration);
    Ok(())
}