//! `Image`
//! 
//! Contains the data needed for parsing file data streams
//! into image-relevant structures.
#![allow(unused_imports)]

mod dimensions;
mod intensity;
mod flim;
mod utils;

/// Functionality for loading arrays with
/// image data (either FLIM format or intensity format)
pub mod load {
    pub (crate) use super::intensity::siff::exports::*;
    pub (crate) use super::flim::histogram::exports::*;
    pub (crate) use super::flim::empirical_lifetime::exports::*;
    pub (crate) use super::flim::phasor::exports::*;
    pub (crate) use super::flim::{load_array_tau_d, load_array_tau_d_registered};
}

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
    /// Returns a borrowed reference to the internal
    /// lifetime data type.
    fn borrow_lifetime(&self) -> &ndarray::Array<Self::Data, D>;
}