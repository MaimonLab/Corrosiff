use std::io::{Error as IOError, ErrorKind as IOErrorKind};
use num_complex::{Complex, c64};
use bytemuck::try_cast_slice;
use itertools::izip;
use ndarray::prelude::*;

use crate::{
    data::image::flim::phasor::unregistered::*,
    data::image::utils::photonwise_op,
    data::image::dimensions::{macros::*, roll_inplace, roll},
    CorrosiffError,
};

#[binrw::parser(reader, endian)]
pub fn _load_flim_intensity_phasor_compressed_registered<T : Into<u64>>(
    phasor_array : &mut ArrayViewMut2<Complex<f64>>,
    intensity_array : &mut ArrayViewMut2<u16>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    cos_lookup : &ArrayView1<f64>,
    sin_lookup : &ArrayView1<f64>,
    registration : (i32, i32),
    ) -> Result<(), CorrosiffError> {
    
    _load_flim_intensity_phasor_compressed(
        reader, endian,
        (phasor_array, intensity_array, strip_byte_counts.into(), ydim, xdim, cos_lookup, sin_lookup)
    )?;

    roll_inplace(phasor_array, registration);
    roll_inplace(intensity_array, registration);
    
    Ok(())
}


#[binrw::parser(reader)]
pub fn _load_flim_intensity_phasor_raw_registered<T : Into<u64>>(
    phasor_array : &mut ArrayViewMut2<Complex<f64>>,
    intensity_array : &mut ArrayViewMut2<u16>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    cos_lookup : &ArrayView1<f64>,
    sin_lookup : &ArrayView1<f64>,
    registration : (i32, i32),
    ) -> Result<(), CorrosiffError> {
 
    photonwise_op!(
        reader,
        strip_byte_counts,
        |siffphoton| {
            let y = photon_to_y!(*siffphoton, registration.0, ydim);
            let x = photon_to_x!(*siffphoton, registration.1, xdim);
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
pub fn _sum_mask_phasor_intensity_raw_registered<T : Into<u64>>(
    mask : &ArrayView2<bool>,
    phasor_sum : &mut Complex<f64>,
    intensity_sum : &mut u64,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    &cos_lookup : &ArrayView1<f64>,
    &sin_lookup : &ArrayView1<f64>,
    registration : (i32, i32),
) -> Result<(), CorrosiffError> {

    photonwise_op!(
        reader,
        strip_byte_counts,
        |siffphoton| {
            let y = photon_to_y!(*siffphoton, registration.0, ydim);
            let x = photon_to_x!(*siffphoton, registration.1, xdim);
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
pub fn _sum_masks_phasor_intensity_raw_registered<T : Into<u64>>(
    masks : &ArrayView3<bool>,
    phasor_sum : &mut ArrayViewMut1<Complex<f64>>,
    intensity_sum : &mut ArrayViewMut1<u64>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    &cos_lookup : &ArrayView1<f64>,
    &sin_lookup : &ArrayView1<f64>,
    registration : (i32, i32),
) -> Result<(), CorrosiffError> {

    photonwise_op!(
        reader,
        strip_byte_counts,
        |siffphoton| {
            let y = photon_to_y!(*siffphoton, registration.0, ydim);
            let x = photon_to_x!(*siffphoton, registration.1, xdim);
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

#[binrw::parser(reader, endian)]
pub fn _sum_mask_phasor_intensity_compressed_registered<T : Into<u64>>(
    mask : &ArrayView2<bool>,
    phasor_sum : &mut Complex<f64>,
    intensity_sum : &mut u64,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    cos_lookup : &ArrayView1<f64>,
    sin_lookup : &ArrayView1<f64>,
    registration : (i32, i32),
) -> Result<(), CorrosiffError> {

    let mask_rolled = roll(mask, (-registration.0, -registration.1));
    _sum_mask_phasor_intensity_compressed(reader, endian, 
        (&mask_rolled.view(), phasor_sum, intensity_sum, strip_byte_counts.into(), ydim, xdim, cos_lookup, sin_lookup)
    )?;

    Ok(())
}


#[binrw::parser(reader, endian)]
pub fn _sum_masks_phasor_intensity_compressed_registered<T : Into<u64>>(
    masks : &ArrayView3<bool>,
    phasor_sum : &mut ArrayViewMut1<Complex<f64>>,
    intensity_sum : &mut ArrayViewMut1<u64>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    cos_lookup : &ArrayView1<f64>,
    sin_lookup : &ArrayView1<f64>,
    registration : (i32, i32),
) -> Result<(), CorrosiffError> {
    
       // Roll the mask the opposite way and then just call the
    // unregistered version
    let mut masks_rolled = masks.to_owned();
    masks_rolled.axis_iter_mut(Axis(0)).for_each(|mut mask| {
        roll_inplace(&mut mask.view_mut(), (-registration.0, -registration.1));
    });

    _sum_masks_phasor_intensity_compressed(reader, endian,
        (&masks_rolled.view(), phasor_sum, intensity_sum, strip_byte_counts.into(), ydim, xdim, cos_lookup, sin_lookup)
    )?;

    Ok(())
}


#[cfg(test)]
mod tests{

}