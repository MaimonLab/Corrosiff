//! `Image`
//! 
//! Contains the data needed for parsing file data streams
//! into image-relevant structures.
#![allow(unused_imports)]

mod dimensions;
mod intensity;
mod flim;
mod utils;

pub (crate) use intensity::siff::load_array as load_array_intensity;
pub (crate) use intensity::siff::load_array_registered as load_array_intensity_registered;
pub (crate) use intensity::siff::sum_mask as sum_intensity_mask;
pub (crate) use intensity::siff::sum_mask_registered as sum_intensity_mask_registered;
pub (crate) use intensity::siff::sum_masks as sum_intensity_masks;
pub (crate) use intensity::siff::sum_masks_registered as sum_intensity_masks_registered;

pub (crate) use flim::histogram::load_histogram as load_histogram;
pub (crate) use flim::histogram::load_histogram_mask as load_histogram_mask;
pub (crate) use flim::histogram::load_histogram_mask_registered as load_histogram_mask_registered;
pub (crate) use flim::empirical_lifetime::load_flim_empirical_and_intensity_arrays
    as load_flim_empirical_and_intensity_arrays;
pub (crate) use flim::empirical_lifetime::load_flim_empirical_and_intensity_arrays_registered
    as load_flim_empirical_and_intensity_arrays_registered;
pub (crate) use flim::empirical_lifetime::sum_lifetime_intensity_mask;
pub (crate) use flim::empirical_lifetime::sum_lifetime_intensity_mask_registered;
pub (crate) use flim::empirical_lifetime::sum_lifetime_intensity_masks;
pub (crate) use flim::empirical_lifetime::sum_lifetime_intensity_masks_registered;

pub (crate) use dimensions::{Dimensions, DimensionsError, roll};

use ndarray;

/// `Image` is a trait that defines the methods that
/// image-like structs should implement. Maybe.
trait Image<D> {
    type Data : ndarray::RawData;
    /// Returns a borrowed reference to the internal 
    /// intensity data of the image.
    fn borrow_intensity(&self) ->  &ndarray::Array<Self::Data, D>;
}

trait Flim<D> : Image<D> {
    fn borrow_lifetime(&self) -> &ndarray::Array<Self::Data, D>;
}