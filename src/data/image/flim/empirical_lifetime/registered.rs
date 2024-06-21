use std::io::{Error as IOError, ErrorKind as IOErrorKind};
use ndarray::prelude::*;
use bytemuck::try_cast_slice;

use crate::{
    data::image::{
            flim::empirical_lifetime::unregistered::{
                _load_flim_array_empirical_compressed,
                _load_flim_intensity_empirical_compressed,
            },
            dimensions::{
                macros::*,
                roll_inplace,
            },
        },
    CorrosiffError,
};

/// A testing and debugging private method --
/// note how this function needs to compute the
/// intensity anyways! Hard to think of a reason to
/// use it.
#[binrw::parser(reader, endian)]
pub fn _load_flim_array_empirical_compressed_registered<T: Into<u64>>(
    array : &mut ArrayViewMut2<f64>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    registration : (i32, i32),
    ) -> Result<(), CorrosiffError> {
    
    _load_flim_array_empirical_compressed(
        reader, endian,
        (array, strip_byte_counts, ydim, xdim)
    )?;

    roll_inplace(array, registration);
    Ok(())
}

/// A testing and debugging private method --
/// note how this function needs to compute the
/// intensity anyways! Hard to think of a reason to
/// use it.
#[binrw::parser(reader)]
pub fn _load_flim_array_empirical_uncompressed_registered<T: Into<u64>>(
    array : &mut ArrayViewMut2<f64>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    registration : (i32, i32),
    ) -> Result<(), CorrosiffError> {
        
    // let bytes = strip_byte_counts.into();
    // let mut photon_stream : Vec<u8> = vec![0; (8*((bytes/8) as usize)) as usize];

    let mut photon_stream : Vec<u8> = vec![0; strip_byte_counts.into() as usize];
    reader.read_exact(&mut photon_stream)?;

    let photons_converted = try_cast_slice::<u8, u64>(&photon_stream).map_err(
        |err| IOError::new(IOErrorKind::InvalidData, err)
    )?;

    let mut intensity = Array2::<u16>::zeros(
        (ydim as usize, xdim as usize)
    );

    photons_converted.iter().for_each(|&photon| {
        let y = photon_to_y!(photon, registration.0, ydim);
        let x = photon_to_x!(photon, registration.1, xdim);
        array[[y, x]] += photon_to_tau_FLOAT!(photon);
        intensity[[y, x]] += 1;
    });

    intensity.iter().zip(array.iter_mut()).for_each(|(intensity, pixel)| {
        *pixel /= *intensity as f64;
    });

    Ok(())
}


#[binrw::parser(reader, endian)]
pub fn _load_flim_intensity_empirical_compressed_registered<T : Into<u64>>(
    lifetime_array : &mut ArrayViewMut2<f64>,
    intensity_array : &mut ArrayViewMut2<u16>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    registration : (i32, i32),
    ) -> Result<(), CorrosiffError> {
    _load_flim_intensity_empirical_compressed(
        reader, endian,
        (lifetime_array, intensity_array, strip_byte_counts.into(), ydim, xdim)
    )?;

    roll_inplace(lifetime_array, registration);
    roll_inplace(intensity_array, registration);
    
    Ok(())
}


#[binrw::parser(reader)]
pub fn _load_flim_intensity_empirical_uncompressed_registered<T : Into<u64>>(
    lifetime_array : &mut ArrayViewMut2<f64>,
    intensity_array : &mut ArrayViewMut2<u16>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    registration : (i32, i32),
    ) -> Result<(), CorrosiffError> {

        
    // let bytes = strip_byte_counts.into();
    // let mut photon_stream : Vec<u8> = vec![0; (8*((bytes/8) as usize)) as usize];
    let mut photon_stream : Vec<u8> = vec![0; strip_byte_counts.into() as usize];
    reader.read_exact(&mut photon_stream)?;

    let photons_converted = try_cast_slice::<u8, u64>(&photon_stream).map_err(
        |err| IOError::new(IOErrorKind::InvalidData, err)
    )?;

    photons_converted.iter().for_each(|&photon| {
        let y = photon_to_y!(photon, registration.0, ydim);
        let x = photon_to_x!(photon, registration.1, xdim);
        lifetime_array[[y, x]] += photon_to_tau_FLOAT!(photon);
        intensity_array[[y, x]] += 1;
    });

    intensity_array.iter().zip(lifetime_array.iter_mut()).for_each(|(intensity, pixel)| {
        *pixel /= *intensity as f64;
    });


    Ok(())
}