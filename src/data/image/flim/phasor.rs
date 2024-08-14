//! Methods in this submodule deal with extracting a pixelwise
//! or ROI-wide phasor from the data stored in a frame of a `.siff` file.

mod unregistered;
mod registered;

use unregistered::{
    _load_flim_intensity_phasor_compressed,
    _load_flim_intensity_phasor_raw,
    _sum_mask_phasor_intensity_compressed,
    _sum_mask_phasor_intensity_raw,
    _sum_masks_phasor_intensity_compressed,
    _sum_masks_phasor_intensity_raw,
};

use registered::{
    _load_flim_intensity_phasor_compressed_registered,
    _load_flim_intensity_phasor_raw_registered,
    _sum_mask_phasor_intensity_compressed_registered,
    _sum_mask_phasor_intensity_raw_registered,
    _sum_masks_phasor_intensity_compressed_registered,
    _sum_masks_phasor_intensity_raw_registered,
};

use std::io::{Error as IOError, ErrorKind as IOErrorKind};
use ndarray::prelude::*;
use num_complex::Complex;
use binrw::io::{Read, Seek};
use crate::{
    tiff::{IFD, TiffTagID::{StripOffsets, StripByteCounts, Siff}, Tag},
    data::image::utils::load_array_from_siff,
    CorrosiffError
};

/// For within-crate use
pub (crate) mod exports {
    pub (crate) use super::{
        load_flim_phasor_and_intensity_arrays,
        load_flim_phasor_and_intensity_arrays_registered,
        sum_phasor_intensity_mask,
        sum_phasor_intensity_mask_registered,
        sum_phasor_intensity_masks,
        sum_phasor_intensity_masks_registered
    };
}

/// Loads intensity and FLIM phasor arrays from the frame
/// pointed to by the IFD. The reader is returned to its original position.
/// 
/// ## Arguments
/// 
/// * `reader` - The reader with access to the siff file
/// (implements `Read` + `Seek`)
/// 
/// * `ifd` - The IFD pointing to the frame to load the lifetime and intensity
/// data from
/// 
/// * `phasor` - The array to load the lifetime into (2d view for one frame)
/// 
/// * `intensity` - The array to load the intensity into (2d view for one frame)
/// 
/// * `cos_lookup` - The lookup table for the cosine of the phasor based on
/// histogram arrival time
/// 
/// * `sin_lookup` - The lookup table for the sine of the phasor based on
/// histogram arrival time
/// 
/// ## Example
/// 
/// ```rust, ignore
/// use ndarray::prelude::*;
/// use std::fs::File;
/// TODO: Write me!
/// ```
/// 
pub fn load_flim_phasor_and_intensity_arrays<I : IFD, ReaderT: Read + Seek> (
    reader : &mut ReaderT,
    ifd : &I,
    phasor_data : &mut ArrayViewMut2<Complex<f64>>,
    intensity_data : &mut ArrayViewMut2<u16>,
    cos_lookup : &ArrayView1<f64>,
    sin_lookup : &ArrayView1<f64>
) -> Result<(), CorrosiffError> {
    load_array_from_siff!(
        reader,
        ifd,
        (
            _load_flim_intensity_phasor_raw,
            (
                phasor_data,
                intensity_data,
                ifd.get_tag(StripByteCounts).unwrap().value(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                cos_lookup,
                sin_lookup
            )
        ),
        (
            _load_flim_intensity_phasor_compressed,
            (
                phasor_data,
                intensity_data,
                ifd.get_tag(StripByteCounts).unwrap().value(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                cos_lookup,
                sin_lookup
            )
        )
    )
}

/// Loads intensity and FLIM phasor arrays from the frame
/// pointed to by the IFD. The reader is returned to its original position.
/// Registers the frame during reading.
/// 
/// ## Arguments
/// 
/// * `reader` - The reader with access to the siff file
/// (implements `Read` + `Seek`)
/// 
/// * `ifd` - The IFD pointing to the frame to load the lifetime and intensity
/// data from
/// 
/// * `phasor` - The array to load the lifetime into (2d view for one frame)
/// 
/// * `intensity` - The array to load the intensity into (2d view for one frame)
/// 
/// * `cos_lookup` - The lookup table for the cosine of the phasor based on
/// histogram arrival time
/// 
/// * `sin_lookup` - The lookup table for the sine of the phasor based on
/// histogram arrival time
/// 
/// * `registration` - The registration to apply to the frame
/// 
/// ## Example
/// 
/// ```rust, ignore
/// use ndarray::prelude::*;
/// use std::fs::File;
/// TODO: Write me!
/// ```
/// 
pub fn load_flim_phasor_and_intensity_arrays_registered<I : IFD, ReaderT: Read + Seek> (
    reader : &mut ReaderT,
    ifd : &I,
    phasor_data : &mut ArrayViewMut2<Complex<f64>>,
    intensity_data : &mut ArrayViewMut2<u16>,
    cos_lookup : &ArrayView1<f64>,
    sin_lookup : &ArrayView1<f64>,
    registration : (i32, i32)
) -> Result<(), CorrosiffError> {
    load_array_from_siff!(
        reader,
        ifd,
        (
            _load_flim_intensity_phasor_raw_registered,
            (
                phasor_data,
                intensity_data,
                ifd.get_tag(StripByteCounts).unwrap().value(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                cos_lookup,
                sin_lookup,
                registration
            )
        ),
        (
            _load_flim_intensity_phasor_compressed_registered,
            (
                phasor_data,
                intensity_data,
                ifd.get_tag(StripByteCounts).unwrap().value(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                cos_lookup,
                sin_lookup,
                registration
            )
        )
    )
}

// Applies a mask to the frame of interest and computes the empirical
/// lifetime across all pixels in the mask and the total intensity
/// within the mask, loading the arguments provided in place.
/// 
/// ## Arguments
/// 
/// * `reader` - The reader with access to the siff file (implements
/// `Read` + `Seek`)
/// 
/// * `ifd` - The IFD pointing to the frame to load the lifetime and intensity
/// data from
/// 
/// * `phasor` - The value to load the computed phasor into
/// 
/// * `intensity` - The value to load the computed intensity into
/// 
/// * `roi` - The mask to apply to the frame
/// 
/// * `cos_lookup` - The lookup table for the cosine of the phasor based on
/// histogram arrival time
/// 
/// * `sin_lookup` - The lookup table for the sine of the phasor based on
/// histogram arrival time
/// 
/// ## Example
/// 
/// ```rust, ignore
/// use ndarray::prelude::*;
/// use std::fs::File;
/// TODO:
/// ```
pub fn sum_phasor_intensity_mask< I : IFD, ReaderT : Read + Seek>(
    reader : &mut ReaderT,
    ifd : &I,
    phasor : &mut Complex<f64>,
    intensity : &mut u64,
    roi : &ArrayView2<bool>,
    cos_lookup : &ArrayView1<f64>,
    sin_lookup : &ArrayView1<f64>
) -> Result<(), CorrosiffError>{
    load_array_from_siff!(
        reader,
        ifd,
        (
            _sum_mask_phasor_intensity_raw,
            (   
                &roi,
                phasor,
                intensity,
                ifd.get_tag(StripByteCounts).unwrap().value().into(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                cos_lookup,
                sin_lookup
            )
        ),
        (
            _sum_mask_phasor_intensity_compressed,
            (
                &roi,
                phasor,
                intensity,
                ifd.get_tag(StripByteCounts).unwrap().value().into(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                cos_lookup,
                sin_lookup
            )
        )
    )
}

// Applies a mask to the frame of interest and computes the empirical
/// lifetime across all pixels in the mask and the total intensity
/// within the mask, loading the arguments provided in place. The
/// frame is registered during reading.
/// 
/// ## Arguments
/// 
/// * `reader` - The reader with access to the siff file (implements
/// `Read` + `Seek`)
/// 
/// * `ifd` - The IFD pointing to the frame to load the lifetime and intensity
/// data from
/// 
/// * `phasor` - The value to load the computed phasor into
/// 
/// * `intensity` - The value to load the computed intensity into
/// 
/// * `roi` - The mask to apply to the frame
/// 
/// * `cos_lookup` - The lookup table for the cosine of the phasor based on
/// histogram arrival time
/// 
/// * `sin_lookup` - The lookup table for the sine of the phasor based on
/// histogram arrival time
/// 
/// * `registration` - The registration to apply to the frame
/// 
/// ## Example
/// 
/// ```rust, ignore
/// use ndarray::prelude::*;
/// use std::fs::File;
/// TODO:
/// ```
pub fn sum_phasor_intensity_mask_registered< I :IFD, ReaderT: Read + Seek>(
    reader : &mut ReaderT,
    ifd : &I,
    phasor : &mut Complex<f64>,
    intensity : &mut u64,
    roi : &ArrayView2<bool>,
    cos_lookup : &ArrayView1<f64>,
    sin_lookup : &ArrayView1<f64>,
    registration : (i32, i32)
) -> Result<(), CorrosiffError>{
    load_array_from_siff!(
        reader,
        ifd,
        (
            _sum_mask_phasor_intensity_raw_registered,
            (   
                &roi,
                phasor,
                intensity,
                ifd.get_tag(StripByteCounts).unwrap().value().into(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                cos_lookup,
                sin_lookup,
                registration
            )
        ),
        (
            _sum_mask_phasor_intensity_compressed_registered,
            (
                &roi,
                phasor,
                intensity,
                ifd.get_tag(StripByteCounts).unwrap().value().into(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                cos_lookup,
                sin_lookup,
                registration
            )
        )
    )
}

/// Applies a series of masks to the frame of interest and computes the
/// phasor representation of the photon arrival time histogram within
/// each mask of interest, storing it in an argument vector
/// 
/// ## Arguments
/// 
/// * `reader` - The reader with access to the siff file (implements
/// `Read` + `Seek`)
/// 
/// * `ifd` - The IFD pointing to the frame to load the lifetime and intensity
/// data from
/// 
/// * `phasor` - The array to load the computed phasors into
/// 
/// * `intensity` - The array to load the computed intensities into
/// 
/// * `rois` - The masks to apply to the frame
/// 
/// * `cos_lookup` - The lookup table for the cosine of the phasor based on
/// histogram arrival time
/// 
/// * `sin_lookup` - The lookup table for the sine of the phasor based on
/// histogram arrival time
/// 
/// ## Example
/// 
/// 
pub fn sum_phasor_intensity_masks< I : IFD, ReaderT : Read + Seek>(
    reader : &mut ReaderT,
    ifd : &I,
    phasor : &mut ArrayViewMut1<Complex<f64>>,
    intensity : &mut ArrayViewMut1<u64>,
    rois : &ArrayView3<bool>,
    cos_lookup : &ArrayView1<f64>,
    sin_lookup : &ArrayView1<f64>
) -> Result<(), CorrosiffError>{
    load_array_from_siff!(
        reader,
        ifd,
        (
            _sum_masks_phasor_intensity_raw,
            (   
                &rois,
                phasor,
                intensity,
                ifd.get_tag(StripByteCounts).unwrap().value().into(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                cos_lookup,
                sin_lookup
            )
        ),
        (
            _sum_masks_phasor_intensity_compressed,
            (
                &rois,
                phasor,
                intensity,
                ifd.get_tag(StripByteCounts).unwrap().value().into(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                cos_lookup,
                sin_lookup
            )
        )
    )
}

/// Applies a series of masks to the frame of interest and computes the
/// phasor representation of the photon arrival time histogram within
/// each mask of interest, storing it in an argument vector. The frame
/// is registered during reading.
/// 
/// ## Arguments
/// 
/// * `reader` - The reader with access to the siff file (implements
/// `Read` + `Seek`)
/// 
/// * `ifd` - The IFD pointing to the frame to load the lifetime and intensity
/// data from
/// 
/// * `phasor` - The array to load the computed phasors into
/// 
/// * `intensity` - The array to load the computed intensities into
/// 
/// * `rois` - The masks to apply to the frame
/// 
/// * `cos_lookup` - The lookup table for the cosine of the phasor based on
/// 
/// * `sin_lookup` - The lookup table for the sine of the phasor based on
/// 
/// * `registration` - The registration to apply to the frame
/// 
/// ## Example
/// 
/// 
pub fn sum_phasor_intensity_masks_registered<I:IFD, ReaderT: Read + Seek>(
    reader : &mut ReaderT,
    ifd : &I,
    phasor : &mut ArrayViewMut1<Complex<f64>>,
    intensity : &mut ArrayViewMut1<u64>,
    rois : &ArrayView3<bool>,
    cos_lookup : &ArrayView1<f64>,
    sin_lookup : &ArrayView1<f64>,
    registration : (i32, i32)
) -> Result<(), CorrosiffError>{
    load_array_from_siff!(
        reader,
        ifd,
        (
            _sum_masks_phasor_intensity_raw_registered,
            (   
                &rois,
                phasor,
                intensity,
                ifd.get_tag(StripByteCounts).unwrap().value().into(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                cos_lookup,
                sin_lookup,
                registration
            )
        ),
        (
            _sum_masks_phasor_intensity_compressed_registered,
            (
                &rois,
                phasor,
                intensity,
                ifd.get_tag(StripByteCounts).unwrap().value().into(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                cos_lookup,
                sin_lookup,
                registration
            )
        )
    )
}