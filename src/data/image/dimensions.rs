//! Code in this submodule deals strictly with attention to
//! image dimensions and the types of things that can go wrong
//! with `Dimensions`.
//! 

use crate::tiff::IFD;

// Lowest 32 bits are the tau coordinate
pub const SIFF_TAU_MASK : u64 = (1<<32) - 1;
/// Highest 16 bits are the y coordinate
pub const SIFF_YMASK : u64 = ((1<<63) | ((1<<63) - 1)) & !((1<<48)-1);
/// Bits 32-48 bits are the x coordinate
pub const SIFF_XMASK : u64 = ((1<<48)- 1) & !((1<<32)-1);


/// `Dimensions` is a simple struct that holds the dimensions
/// of a frame
/// 
/// `xdim` is the width of the frame
/// `ydim` is the height of the frame
#[derive(PartialEq, Debug, Clone)]
pub struct Dimensions {
    pub xdim : u64,
    pub ydim : u64
}

#[derive(Debug, Clone)]
pub enum DimensionsError {
    MismatchedDimensions{required : Dimensions, requested: Dimensions},
    NoConsistentDimensions,
    IncorrectFrames,
}

impl Dimensions {
    pub fn new(xdim : u64, ydim : u64) -> Dimensions {
        Dimensions {
            xdim,
            ydim,
        }
    }

    pub fn from_ifd<'a, I : IFD>(ifd : &I)-> Dimensions {
        Dimensions {
            xdim : ifd.width().unwrap().into(),
            ydim : ifd.height().unwrap().into(),
        }
    }
    
    /// Returns the dimensions as a tuple (y, x)
    pub fn to_tuple(&self) -> (u64, u64) {
        (self.ydim, self.xdim)
    }
}

impl std::error::Error for DimensionsError {}

impl std::fmt::Display for DimensionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DimensionsError::MismatchedDimensions{required, requested} => {
                write!(f, "Mismatched dimensions. Requested: ({}, {}), Required: ({}, {})",
                    requested.xdim, requested.ydim, required.xdim, required.ydim)
            },
            DimensionsError::NoConsistentDimensions => {
                write!(f, "Requested data did not have consistent dimensions.")
            },
            DimensionsError::IncorrectFrames => {
                write!(f, "Requested frames are out of bounds.")
            }
        }
    }
}