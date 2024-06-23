use ndarray::prelude::*;
use itertools::izip;

use crate::{
    data::image::{
            utils::photonwise_op,
            flim::empirical_lifetime::unregistered::{
                _load_flim_array_empirical_compressed,
                _load_flim_intensity_empirical_compressed,
                _sum_mask_empirical_intensity_compressed,
                _sum_masks_empirical_intensity_compressed,
            },
            dimensions::{
                macros::*,
                roll_inplace,
                roll,
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
        
    let mut intensity = Array2::<u16>::zeros(
        (ydim as usize, xdim as usize)
    );

    photonwise_op!(
        reader,
        strip_byte_counts,
        |siffphoton| {
            let y = photon_to_y!(*siffphoton, registration.0, ydim);
            let x = photon_to_x!(*siffphoton, registration.1, xdim);
            array[[y, x]] += photon_to_tau_FLOAT!(*siffphoton);
            intensity[[y, x]] += 1;
        }
    );

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

    photonwise_op!(
        reader,
        strip_byte_counts,
        |siffphoton| {
            let y = photon_to_y!(*siffphoton, registration.0, ydim);
            let x = photon_to_x!(*siffphoton, registration.1, xdim);
            lifetime_array[[y, x]] += photon_to_tau_FLOAT!(*siffphoton);
            intensity_array[[y, x]] += 1;
        }
    );

    intensity_array.iter().zip(lifetime_array.iter_mut()).for_each(|(intensity, pixel)| {
        *pixel /= *intensity as f64;
    });


    Ok(())
}

#[binrw::parser(reader)]
pub fn _sum_mask_empirical_intensity_raw_registered<T : Into<u64>>(
    mask : &ArrayView2<bool>,
    lifetime_sum : &mut f64,
    intensity_sum : &mut u64,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    registration : (i32, i32),
) -> Result<(), CorrosiffError> {

    photonwise_op!(
        reader,
        strip_byte_counts,
        |siffphoton| {
            let y = photon_to_y!(*siffphoton, registration.0, ydim);
            let x = photon_to_x!(*siffphoton, registration.1, xdim);
            *lifetime_sum += photon_to_tau_FLOAT!(*siffphoton)
                * (mask[[y, x]] as u64 as f64);
            *intensity_sum += mask[[y, x]] as u64;
        }
    );

    *lifetime_sum /= *intensity_sum as f64;

    Ok(())
}

#[binrw::parser(reader)]
pub fn _sum_masks_empirical_intensity_raw_registered<T : Into<u64>>(
    masks : &ArrayView3<bool>,
    lifetime_sum : &mut ArrayViewMut1<f64>,
    intensity_sum : &mut ArrayViewMut1<u64>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
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
                lifetime_sum.iter_mut(),
                intensity_sum.iter_mut()
            ).for_each(|(mask, lifetime_sum, intensity_sum)| {
                *lifetime_sum += photon_to_tau_FLOAT!(*siffphoton)
                    * (mask[[y, x]] as u64 as f64);
                *intensity_sum += mask[[y, x]] as u64;
            });
        }
    );

    izip!(
        lifetime_sum.iter_mut(),
        intensity_sum.iter()
    ).for_each(|(lifetime_sum, intensity_sum)| {
        *lifetime_sum /= *intensity_sum as f64;
    });
    
    Ok(())
}

#[binrw::parser(reader, endian)]
pub fn _sum_mask_empirical_intensity_compressed_registered<T : Into<u64>>(
    mask : &ArrayView2<bool>,
    lifetime_sum : &mut f64,
    intensity_sum : &mut u64,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    registration : (i32, i32),
) -> Result<(), CorrosiffError> {

    // Roll the mask the opposite way and then just call the
    // unregistered version
    let mask_rolled = roll(mask, (-registration.0, -registration.1));
    _sum_mask_empirical_intensity_compressed(
        reader,
        endian,
        (
            &mask_rolled.view(),
            lifetime_sum,
            intensity_sum,
            strip_byte_counts.into(),
            ydim,
            xdim,
        )
    )?;

    Ok(())
}

#[binrw::parser(reader,endian)]
pub fn _sum_masks_empirical_intensity_compressed_registered<T : Into<u64>>(
    masks : &ArrayView3<bool>,
    lifetime_sum : &mut ArrayViewMut1<f64>,
    intensity_sum : &mut ArrayViewMut1<u64>,
    strip_byte_counts : T,
    ydim : u32,
    xdim : u32,
    registration : (i32, i32),
) -> Result<(), CorrosiffError> {

    // Roll the mask the opposite way and then just call the
    // unregistered version
    let mut masks_rolled = masks.to_owned();
    masks_rolled.axis_iter_mut(Axis(0)).for_each(|mut mask| {
        roll_inplace(&mut mask.view_mut(), (-registration.0, -registration.1));
    });

    _sum_masks_empirical_intensity_compressed(
        reader,
        endian,
        (
            &masks_rolled.view(),
            lifetime_sum,
            intensity_sum,
            strip_byte_counts.into(),
            ydim,
            xdim,
        )
    )?;

    Ok(())
}