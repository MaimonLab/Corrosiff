mod parallelize_op;

pub (super) use parallelize_op::parallelize_op as parallelize_op;
//pub (super) use parallelize_op::registration_dependent_op;

use crate::data::image::DimensionsError;


/// Errors that can occur due to frame processing
/// problems, either from the file reader (the
/// `IOError` variant) or the values of the requested
/// frames (e.g. incompatible dimensions or out of bounds).
#[derive(Debug)]
pub enum FramesError{
    FormatError(String),
    DimensionsError(DimensionsError),
    IOError(std::io::Error),
    RegistrationFramesMissing,
}

impl From<DimensionsError> for FramesError {
    fn from(err : DimensionsError) -> Self {
        FramesError::DimensionsError(err)
    }
}

impl From<std::io::Error> for FramesError {
    fn from(err : std::io::Error) -> Self {
        FramesError::IOError(err)
    }
}

impl From<binrw::Error> for FramesError {
    fn from(err : binrw::Error) -> Self {
        FramesError::IOError(
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                err.to_string()
            )
        )
    }
}

impl std::error::Error for FramesError {}

impl std::fmt::Display for FramesError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FramesError::DimensionsError(err) => {
                write!(f, "DimensionsError: {}", err)
            },
            FramesError::IOError(err) => {
                write!(f, "IOError: {}", err)
            },
            FramesError::RegistrationFramesMissing => {
                write!(f, "Registration frames missing")
            },
            FramesError::FormatError(err) => {
                write!(f, "FormatError: {}", err)
            }
        }
    }
}

