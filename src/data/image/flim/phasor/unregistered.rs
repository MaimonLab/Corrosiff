use std::io::{Error as IOError, ErrorKind as IOErrorKind};
use num_complex::{Complex, c64};
use bytemuck::try_cast_slice;
use itertools::izip;
use ndarray::prelude::*;

use crate::{
    data::image::utils::photonwise_op,
    data::image::dimensions::macros::*,
    CorrosiffError,
};

#[binrw::parser(reader)]
pub fn _load_flim_intensity_phasor_compressed<T : Into<u64>>(
    phasor_array : &mut ArrayViewMut2<Complex<f64>>,
    intensity_array : &mut ArrayViewMut2<u16>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    cos_lookup : &ArrayView1<f64>,
    sin_lookup : &ArrayView1<f64>,
    ) -> Result<(), CorrosiffError> {
    
    let mut intensity_read : Vec<u8> = vec![0;
        ((ydim * xdim)*std::mem::size_of::<u16>() as u32) as usize
    ];

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

    let mut arrival_time_read : Vec<u8> = vec![0; strip_byte_counts.into() as usize];
    reader.read_exact(&mut arrival_time_read)?;

    let arrival_times = try_cast_slice::<u8, u16>(&arrival_time_read).map_err(
        |err| IOError::new(IOErrorKind::InvalidData, err)
    )?;

    let mut arrival_time_pointer : usize = 0;

    let lookup_len = cos_lookup.len();

    intensity_array.iter().zip(phasor_array.iter_mut()).for_each( | (intensity, pixel) | {
        arrival_times[arrival_time_pointer..arrival_time_pointer+*intensity as usize].iter()
            .for_each(|&arrival_time| {
                *pixel += Complex::new(
                    cos_lookup[arrival_time as usize % lookup_len],
                    sin_lookup[arrival_time as usize % lookup_len]
                );
            }
        );
        *pixel /= *intensity as f64;
        arrival_time_pointer += *intensity as usize;
    });
    Ok(())
}


#[binrw::parser(reader)]
pub fn _load_flim_intensity_phasor_raw<T : Into<u64>>(
    phasor_array : &mut ArrayViewMut2<Complex<f64>>,
    intensity_array : &mut ArrayViewMut2<u16>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    cos_lookup : &ArrayView1<f64>,
    sin_lookup : &ArrayView1<f64>,
    ) -> Result<(), CorrosiffError> {
 
    photonwise_op!(
        reader,
        strip_byte_counts,
        |siffphoton| {
            let y = photon_to_y!(*siffphoton, 0, ydim);
            let x = photon_to_x!(*siffphoton, 0, xdim);
            let tau = photon_to_tau_USIZE!(*siffphoton);
            phasor_array[[y, x]] += Complex::new(
                cos_lookup[tau as usize % cos_lookup.len()],
                sin_lookup[tau as usize % sin_lookup.len()]
            );
            intensity_array[[y, x]] += 1;
        }
    );

    intensity_array.iter().zip(phasor_array.iter_mut()).for_each(|(intensity, pixel)| {
        *pixel /= *intensity as f64;
    });

    Ok(())
}

#[binrw::parser(reader)]
pub fn _sum_mask_phasor_intensity_raw<T : Into<u64>>(
    mask : &ArrayView2<bool>,
    phasor_sum : &mut Complex<f64>,
    intensity_sum : &mut u64,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    &cos_lookup : &ArrayView1<f64>,
    &sin_lookup : &ArrayView1<f64>,
) -> Result<(), CorrosiffError> {

    photonwise_op!(
        reader,
        strip_byte_counts,
        |siffphoton| {
            let y = photon_to_y!(*siffphoton, 0, ydim);
            let x = photon_to_x!(*siffphoton, 0, xdim);
            let tau = photon_to_tau_USIZE!(*siffphoton);
            *phasor_sum += Complex::new(
                cos_lookup[tau as usize % cos_lookup.len()],
                sin_lookup[tau as usize % sin_lookup.len()]
            ) * (mask[[y, x]] as u64 as f64);
            *intensity_sum += mask[[y, x]] as u64;
        }
    );

    *phasor_sum /= *intensity_sum as f64;

    Ok(())
}

#[binrw::parser(reader)]
pub fn _sum_masks_phasor_intensity_raw<T : Into<u64>>(
    masks : &ArrayView3<bool>,
    phasor_sum : &mut ArrayViewMut1<Complex<f64>>,
    intensity_sum : &mut ArrayViewMut1<u64>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    &cos_lookup : &ArrayView1<f64>,
    &sin_lookup : &ArrayView1<f64>,
) -> Result<(), CorrosiffError> {

    photonwise_op!(
        reader,
        strip_byte_counts,
        |siffphoton| {
            let y = photon_to_y!(*siffphoton, 0, ydim);
            let x = photon_to_x!(*siffphoton, 0, xdim);
            izip!(
                masks.axis_iter(Axis(0)),
                phasor_sum.iter_mut(),
                intensity_sum.iter_mut()
            ).for_each(
                |(mask, lifetime_sum, intensity_sum)| {
                    *intensity_sum += mask[[y, x]] as u64;
                    let tau = photon_to_tau_USIZE!(*siffphoton);
                    *lifetime_sum += mask[[y, x]] as u64 as f64 
                    * Complex::new(
                        cos_lookup[tau as usize % cos_lookup.len()],
                        sin_lookup[tau as usize % sin_lookup.len()]
                    );
                }
            );
        }
    );

    izip!(phasor_sum.iter_mut(), intensity_sum.iter()).for_each(
        |(lifetime_sum, intensity_sum)| {
            *lifetime_sum /= *intensity_sum as f64;
        }
    );

    Ok(())
}

#[binrw::parser(reader)]
pub fn _sum_mask_phasor_intensity_compressed<T : Into<u64>>(
    mask : &ArrayView2<bool>,
    phasor_sum : &mut Complex<f64>,
    intensity_sum : &mut u64,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    cos_lookup : &ArrayView1<f64>,
    sin_lookup : &ArrayView1<f64>,
) -> Result<(), CorrosiffError> {

    reader.seek(std::io::SeekFrom::Current(
        -((ydim * xdim * std::mem::size_of::<u16>() as u32) as i64)
    ))?;

    let mut data : Vec<u8> = vec![0;
        ydim as usize * xdim as usize * std::mem::size_of::<u16>()
    ];

    reader.read_exact(&mut data)?;

    let intensity_data = try_cast_slice::<u8, u16>(&data).map_err(
        |err| IOError::new(IOErrorKind::InvalidData, err)
    )?;

    let mut lifetime_data : Vec<u8> = vec![0; strip_byte_counts.into() as usize];
    reader.read_exact(&mut lifetime_data)?;

    let lifetime_data = try_cast_slice::<u8, u16>(&lifetime_data).map_err(
        |err| IOError::new(IOErrorKind::InvalidData, err)
    )?;

    let mut lifetime_pointer : usize = 0;
    
    intensity_data.iter().zip(mask.iter()).for_each(|(intensity, maskpx)| {
        *intensity_sum += (*intensity as u64) * (*maskpx as u64);
        *phasor_sum += (*maskpx as u64 as f64)
            * lifetime_data[lifetime_pointer..lifetime_pointer+*intensity as usize]
            .iter()
            .map(|tau| Complex::new(
                cos_lookup[*tau as usize % cos_lookup.len()],
                sin_lookup[*tau as usize % sin_lookup.len()]
            ))
            .sum::<Complex<f64>>();
        lifetime_pointer += *intensity as usize;
    });

    *phasor_sum /= *intensity_sum as f64;
    Ok(())
}


#[binrw::parser(reader)]
pub fn _sum_masks_phasor_intensity_compressed<T : Into<u64>>(
    masks : &ArrayView3<bool>,
    phasor_sum : &mut ArrayViewMut1<Complex<f64>>,
    intensity_sum : &mut ArrayViewMut1<u64>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    cos_lookup : &ArrayView1<f64>,
    sin_lookup : &ArrayView1<f64>,
) -> Result<(), CorrosiffError> {
    
    reader.seek(std::io::SeekFrom::Current(
        -((ydim * xdim * std::mem::size_of::<u16>() as u32) as i64)
    ))?;

    let mut data : Vec<u8> = vec![0;
        ydim as usize * xdim as usize * std::mem::size_of::<u16>()
    ];

    reader.read_exact(&mut data)?;

    let intensity_data = try_cast_slice::<u8, u16>(&data).map_err(
        |err| IOError::new(IOErrorKind::InvalidData, err)
    )?;

    let mut lifetime_data : Vec<u8> = vec![0; strip_byte_counts.into() as usize];
    reader.read_exact(&mut lifetime_data)?;

    let lifetime_data = try_cast_slice::<u8, u16>(&lifetime_data).map_err(
        |err| IOError::new(IOErrorKind::InvalidData, err)
    )?;

    // The painful process of iterating over the data N times again.
    // I think with a smarter programmer, or more time, one can make this
    // more efficient, but maybe I need to relax and trust the compiler??

    izip!(
        masks.axis_iter(Axis(0)),
        intensity_sum.iter_mut(),
        phasor_sum.iter_mut()
    ).for_each(|(mask, intensity_accumulator, phasor_accumulator)| {
        let mut lifetime_pointer : usize = 0;
        intensity_data.iter().zip(mask.iter()).for_each(|(intensity, maskpx)| {
            *intensity_accumulator += (*intensity as u64) * (*maskpx as u64);
            *phasor_accumulator += (*maskpx as u64 as f64)
                * lifetime_data[lifetime_pointer..lifetime_pointer+*intensity as usize]
                    .iter()
                    .map(|&tau| Complex::new(
                        cos_lookup[tau as usize % cos_lookup.len()],
                        sin_lookup[tau as usize % sin_lookup.len()]
                    ))
                    .sum::<Complex<f64>>();
            lifetime_pointer += *intensity as usize;
        });
        *phasor_accumulator /= *intensity_accumulator as f64;
    });

    Ok(())
}


#[cfg(test)]
mod tests{

}