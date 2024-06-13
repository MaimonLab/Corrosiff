//! `Image`
//! 
//! Contains the data needed for parsing file data streams
//! into image-relevant structures.

mod dimensions;
mod intensity;
mod flim;

pub use intensity::siff::load_array as load_array_intensity;
pub use intensity::siff::load_array_registered as load_array_intensity_registered;

pub use flim::histogram::load_histogram as load_histogram;

pub use dimensions::{Dimensions, DimensionsError};

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