use std::io::{Error as IOError, ErrorKind as IOErrorKind};
use ndarray::prelude::*;
use bytemuck::try_cast_slice;

use crate::{
    data::image::dimensions::macros::*,
    CorrosiffError,
};

/// A testing and debugging private method --
/// note how this function needs to compute the
/// intensity anyways! Hard to think of a reason to
/// use it.
#[binrw::parser(reader)]
pub fn _load_flim_array_empirical_compressed<T: Into<u64>>(
    array : &mut ArrayViewMut2<f64>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    ) -> Result<(), CorrosiffError> {
    
    let mut intensity_read : Vec<u8> = vec![0; ((ydim * xdim)*std::mem::size_of::<u16>() as u32) as usize];
    reader.seek(std::io::SeekFrom::Current(-(intensity_read.len() as i64)))?;

    reader.read_exact(&mut intensity_read)?;
    let intensity = try_cast_slice::<u8, u16>(&intensity_read).map_err(
        |err| IOError::new(IOErrorKind::InvalidData, err)
    )?;

    // All the photons in order as `u16`s
    let mut lifetime_read : Vec<u8> = vec![0; strip_byte_counts.into() as usize];
        
    // let bytes = strip_byte_counts.into();
    // let mut lifetime_read : Vec<u8> = vec![0; (8*((bytes/8) as usize)) as usize];

    reader.read_exact(&mut lifetime_read)?;

    let arrival_times = try_cast_slice::<u8, u16>(&lifetime_read).map_err(
        |err| IOError::new(IOErrorKind::InvalidData, err)
    )?;

    // increments through arrival_times with intensity
    let mut arrival_time_pointer : usize = 0;
    intensity.iter().zip(array.iter_mut()).for_each(|(intensity, pixel)| {
        arrival_times[arrival_time_pointer..arrival_time_pointer+*intensity as usize].iter().for_each(|x| {
            *pixel += *x as f64;
        });
        *pixel /= *intensity as f64;
        arrival_time_pointer += *intensity as usize;
    });

    Ok(())
}

/// A testing and debugging private method --
/// note how this function needs to compute the
/// intensity anyways! Hard to think of a reason to
/// use it.
#[binrw::parser(reader)]
pub fn _load_flim_array_empirical_uncompressed<T : Into<u64>>(
    array : &mut ArrayViewMut2<f64>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
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
        let y = photon_to_y!(photon, 0, ydim);
        let x = photon_to_x!(photon, 0, xdim);
        array[[y, x]] += photon_to_tau_FLOAT!(photon);
        intensity[[y, x]] += 1;
    });

    intensity.iter().zip(array.iter_mut()).for_each(|(intensity, pixel)| {
        *pixel /= *intensity as f64;
    });

    Ok(())
}


#[binrw::parser(reader)]
pub fn _load_flim_intensity_empirical_compressed<T : Into<u64>>(
    lifetime_array : &mut ArrayViewMut2<f64>,
    intensity_array : &mut ArrayViewMut2<u16>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    ) -> Result<(), CorrosiffError> {
    
    let mut intensity_read : Vec<u8> = vec![0; ((ydim * xdim)*std::mem::size_of::<u16>() as u32) as usize];
    reader.seek(std::io::SeekFrom::Current(-(intensity_read.len() as i64)))?;

    reader.read_exact(&mut intensity_read)?;
    
    // Set intensity_array's data to the `u16` version of intensity_read
    let intensity = try_cast_slice::<u8, u16>(&intensity_read).map_err(
        |err| IOError::new(IOErrorKind::InvalidData, err)
    )?;

    intensity_array.assign(
        &Array2::<u16>::from_shape_vec(
            (ydim as usize, xdim as usize),
            intensity.to_vec()
        ).map_err(|err| IOError::new(IOErrorKind::InvalidData, err))?
    );

    // All the photons in order as `u16`s
    let mut lifetime_read : Vec<u8> = vec![0; strip_byte_counts.into() as usize];
    reader.read_exact(&mut lifetime_read)?;

    let arrival_times = try_cast_slice::<u8, u16>(&lifetime_read).map_err(
        |err| IOError::new(IOErrorKind::InvalidData, err)
    )?;

    // increments through arrival_times with intensity
    let mut arrival_time_pointer : usize = 0;
    intensity_array.iter().zip(lifetime_array.iter_mut()).for_each(|(intensity, pixel)| {
        arrival_times[arrival_time_pointer..arrival_time_pointer+*intensity as usize].iter().for_each(|x| {
            *pixel += *x as f64;
        });
        *pixel /= *intensity as f64;
        arrival_time_pointer += *intensity as usize;
    });
    Ok(())
}


#[binrw::parser(reader)]
pub fn _load_flim_intensity_empirical_uncompressed<T : Into<u64>>(
    lifetime_array : &mut ArrayViewMut2<f64>,
    intensity_array : &mut ArrayViewMut2<u16>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    ) -> Result<(), CorrosiffError> {
    
    
    // let bytes = strip_byte_counts.into();
    // let mut photon_stream : Vec<u8> = vec![0; (8*((bytes/8) as usize)) as usize];
    let mut photon_stream : Vec<u8> = vec![0; strip_byte_counts.into() as usize];
    reader.read_exact(&mut photon_stream)?;

    let photons_converted = try_cast_slice::<u8, u64>(&photon_stream).map_err(
        |err| IOError::new(IOErrorKind::InvalidData, err)
    )?;

    photons_converted.iter().for_each(|&photon| {
        let y = photon_to_y!(photon, 0, ydim);
        let x = photon_to_x!(photon, 0, xdim);
        lifetime_array[[y, x]] += photon_to_tau_FLOAT!(photon);
        intensity_array[[y, x]] += 1;
    });

    intensity_array.iter().zip(lifetime_array.iter_mut()).for_each(|(intensity, pixel)| {
        *pixel /= *intensity as f64;
    });

    Ok(())
}