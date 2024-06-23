//! # Corrosiff
//! 
//! `corrosiff` is a library for reading data from `.siff` files
//! or converting the intensity data in `.siff` files to `.tiff` files
//! if you don't care about arrival times and just want images that can
//! be passed into existing pipelines.
//! 
//! Almost all uses will rely on the `SiffReader` struct rather than
//! the methods of the `corrosiff` module itself. The `SiffReader` struct
//! can be constructed with `open_siff` and then used to read frames data,
//! metadata, and other information. Some convenience methods are provided
//! in the `corrosiff` module for converting `.siff` files to `.tiff` files
//! or for reading frames directly for a one-time call if you don't expect
//! multiple interactions with the file.

use std::{
    io::Result as IOResult,
    path::{Path,PathBuf},
    fs::File,
    collections::HashMap,
};

use ndarray::prelude::*;

mod tiff;
mod data;
mod utils;

use crate::data::image::DimensionsError;

pub mod metadata;
pub mod siffreader;

pub use siffreader::{SiffReader, RegistrationDict};
pub use utils::FramesError;
pub use metadata::FrameMetadata;
pub use data::time::ClockBase;

#[derive(Debug)]
pub enum CorrosiffError {
    IOError(std::io::Error),
    FramesError(FramesError),
    DimensionsError(data::image::DimensionsError),
    InvalidClockBase,
    NoSystemTimestamps,
    NotImplementedError,
}

impl From<std::io::Error> for CorrosiffError {
    fn from(err : std::io::Error) -> Self {
        CorrosiffError::IOError(err)
    }
}

impl From<FramesError> for CorrosiffError {
    fn from(err : FramesError) -> Self {
        CorrosiffError::FramesError(err)
    }
}

impl From<DimensionsError> for CorrosiffError {
    fn from(err : DimensionsError) -> Self {
        CorrosiffError::DimensionsError(err)
    }
}

impl std::error::Error for CorrosiffError {}

impl std::fmt::Display for CorrosiffError {
    fn fmt(&self, f : &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CorrosiffError::IOError(err) => write!(f, "IO Error: {}", err),
            CorrosiffError::FramesError(err) => write!(f, "Frames Error: {}", err),
            CorrosiffError::DimensionsError(err) => write!(f, "Dimensions Error: {}", err),
            CorrosiffError::InvalidClockBase => write!(f, "Invalid clock base for function called"),
            CorrosiffError::NoSystemTimestamps => write!(f, "No system clock timestamps for this file"),
            CorrosiffError::NotImplementedError => write!(f, "Not Implemented"),
        }
    }
}

/// Enum for specifying the conversion mode
/// of a `.siff` file to a `.tiff` file.
/// 
/// ## Variants
/// 
/// * `ScanImage` - The standard ScanImage format
/// * `OME` - The OME-TIFF format
/// 
/// ## Examples
/// 
/// ### From string, for argument parsing
/// ```
/// use corrosiff::TiffMode;
///     
/// let mode = TiffMode::from_string_slice("OME");
/// ```
/// 
/// ### From enum
/// ```
/// use corrosiff::TiffMode;
/// 
/// let mode = TiffMode::OME;
/// ```
#[derive(Debug, PartialEq)]
pub enum TiffMode {
    ScanImage,
    OME,
}

impl TiffMode {
    /// `from_string_slice(str)` parses a string slice
    /// to produce a `TiffMode` enum. Useful for argument
    /// parsing from the command line.
    /// 
    /// ## Arguments
    /// 
    /// * `str` - A string slice that holds the name of the mode
    pub fn from_string_slice(str : & str) -> IOResult<TiffMode> {
        match str {
            "ScanImage" => Ok(TiffMode::ScanImage),
            "OME" => Ok(TiffMode::OME),
            _ => Err(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid TiffMode"
                )
            ),
        }
    }
}

/// `open_siff(filename)` opens a `.siff` file
/// or a ScanImage-Flim `.tiff` file, reads the data,
/// and returns a `SiffReader` object.
/// 
/// ## Arguments
/// 
/// * `filename` - A string slice that holds the name of the file to open
/// 
/// ## Example
/// 
/// ```rust, ignore
/// use corrosiff::open_siff;
/// let reader = open_siff("file.siff");
/// ```
pub fn open_siff<P : AsRef<Path>>(filename : P) -> IOResult<siffreader::SiffReader> {
    SiffReader::open(filename)
}

/// `time_axis epoch`
/// 
/// Returns the timestamp of all frames in the `.siff` file
/// in `epoch` time, determined either by the system clock stamps
/// of the microscope computer or by counting laser sync pulses.
/// 
/// Returns a one-dimensional array of `u64` values
/// 
/// ## See also
/// - `time_axis_experiment` for time since acquisition began
/// - `time_axis_epoch_both` to get both system and laser timestamps
/// in one array, which is more useful for self-correction.
/// 
/// ## Errors
/// 
/// * `CorrosiffError::InvalidClockBase` - If the clock base is not
/// `EpochLaser` or `EpochSystem`, this error is returned to signify
/// that the user likely wanted to call `time_axis_experiment` 
/// or `time_axis_epoch_both` instead.
pub fn time_axis_epoch(
    siffreader : &SiffReader,
    units : ClockBase
) -> Result<Array1<u64>, CorrosiffError> {
    match units {
        ClockBase::EpochLaser => {
            siffreader.get_epoch_timestamps_laser(
                siffreader.frames_vec().as_slice()
            )
        },
        ClockBase::EpochSystem => {
            siffreader.get_epoch_timestamps_system(
                siffreader.frames_vec().as_slice()
            )
        },
        _ => Err(CorrosiffError::InvalidClockBase)
    }
}

/// `time_axis_experiment`
/// 
/// Returns the timestamp of all frames in the `.siff` file
/// in `experiment` time, determined by the time since the
/// acquisition began (in seconds).
pub fn time_axis_experiment(
    siffreader : &SiffReader
) -> Result<Array1<f64>, CorrosiffError> {
    siffreader.get_experiment_timestamps(
        siffreader.frames_vec().as_slice()
    )
}

/// `time_axis_epoch_both`
/// 
/// Returns the timestamp of all frames in the `.siff` file
/// in both `epoch` time bases, one computed from counting
/// laser pulses (highly regular but susceptible to drifting from
/// the system clock) and from regular system clock calls (high jitter,
/// but self-correcting via the PTP connection to a master clock). Regressing
/// the two against each other should provide a correcting factor for the
/// laser timestamps to allow highly precise timing of every frame.
/// 
/// ## Returns
/// 
/// * `Result<Array2<u64>, CorrosiffError>` - An `Array2<u64>` containing the
/// timestamps of all frames in the `.siff` file, with the first row
/// being the laser timestamps and the second row being the system timestamps.
/// 
/// ## Errors
/// 
/// * `CorrosiffError::NoSystemTimestamps` - If there are no system timestamps
/// in the `.siff` file, this error is returned.
/// 
/// * `CorrosiffError::DimensionsError(DimensionsError)` - If the requested
/// frames are out of bounds, this error is returned.
pub fn time_axis_epoch_both(
    siffreader : &SiffReader
) -> Result<Array2<u64>, CorrosiffError> {
    siffreader.get_epoch_timestamps_both(siffreader.frames_vec().as_slice())
}

/// `get_frames(path, frames, registration)` returns the intensity data of the
/// specified frames from the `.siff` file, with optional in-place registration.
/// 
/// ## Arguments
/// 
/// * `file_path` - A string slice that holds the name of the file to open
/// * `frames` - A slice of `u64` values specifying the frames to read
/// * `registration` - An optional `HashMap<u64, (i32, i32)>` specifying
/// the pixel shifts for each frame. If not specified, the frames are
/// read unregistered.
/// 
/// ## Returns
/// 
/// * `Result<Array3<u16>, CorrosiffError>` - An `Array3<u16>` containing the
/// intensity data of the specified frames of size `(frames, height, width)`.
/// 
/// ## Example
/// 
/// ```rust, ignore
/// use corrosiff::{open_siff, get_frames};
///    
/// let frames = vec![0, 1, 2, 3];
/// // No pixel shifts
/// let frames_array = get_frames("file.siff", &frames, None).unwrap();
/// 
/// // 256 lines, 128 pixels per line, 4 frames.
/// assert_eq!(frames_array.shape(), (4, 256, 128));
/// ```
/// 
/// ## Errors
/// 
/// * `CorrosiffError::IOError` - If there is an error reading the file or
/// performing any conversion from the binary data, it is returned as an
/// `IOError`
/// 
/// * `CorrosiffError::FramesError` - If there is an error with the frames
/// requested, e.g. inconsistent dimension or an out-of-bounds request,
/// it is returned as a `FramesError` subtype.
pub fn get_frames(
    file_path : &str,
    frames : &[u64],
    registration : Option<&HashMap<u64, (i32, i32)>>
    ) -> Result<Array3<u16>, CorrosiffError>
    {
    let siffreader = SiffReader::open(file_path)?;
    siffreader.get_frames_intensity(frames, registration).map_err(|err| err.into())
}

/// `siff_to_tiff(filename, mode)` converts a `.siff` file
/// to a `.tiff` file, using the specified mode. If the mode
/// is `TiffMode::ScanImage`, the main metadata remains
/// unconverted, and it uses the standard ScanImage format.
/// If the mode is `TiffMode::OME`, the metadata is converted
/// to the OME-TIFF format.
/// 
/// ## Arguments
/// 
/// * `filename` - A string slice that holds the name of the file to open
/// * `mode` - A string slice specifying the `TiffMode` enum to be used
/// * `save_path` - An optional string slice that holds the path
/// to save the converted file. If not specified, the file is saved
/// in the same directory as the original file, with the same name
/// but the extension `.tiff`.
/// 
/// ## Example
/// 
/// ```rust, ignore
/// use corrosiff::{siff_to_tiff, TiffMode};
/// // Produces "file.tiff" in OME-TIFF format (not-yet implemented)
/// siff_to_tiff("file.siff", TiffMode::from_string_slice("OME"), None);
/// // Produces "file2.tiff" in ScanImage format
/// siff_to_tiff("file.siff", TiffMode::from_string_slice("ScanImage"), Some("file2.tiff"));
/// ```
pub fn siff_to_tiff(
    filename : & str,
    _mode : TiffMode,
    save_path : Option<&String>,
    ) -> IOResult<()>{

    let file_path: PathBuf = PathBuf::from(filename);

    // let siffreader = SiffReader::open(file_path.to_str().unwrap())?;

    let save_path: PathBuf = save_path
        .map(PathBuf::from)
        .unwrap_or_else(|| {file_path.with_extension("tiff")});

    File::create(save_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    pub static TEST_FILE_PATH : &str = "/Users/stephen/Desktop/Data/imaging/2024-04/2024-04-17/21Dhh_GCaFLITS/Fly1/BarOnAtTen_1.siff";
    pub const UNCOMPRESSED_FRAME_NUM : usize = 14;
    pub const COMPRESSED_FRAME_NUM : usize = 40;
    
    pub static APPENDED_TEXT_FILE : &str = "/Users/stephen/Desktop/Data/imaging/2024-05/2024-05-27/L2Split_GCaFLITS_KCL/Fly1/KClApplication_1.siff";

    pub static BIG_FILE_PATH :&str = "/Users/stephen/Desktop/Data/imaging/2024-05/2024-05-27/SS02255_greenCamui_alpha/Fly1/PB_1.siff";

    #[test]
    fn test_open_siff() {
        let reader = open_siff("file.siff");
        assert!(reader.is_err());

        let reader = open_siff(TEST_FILE_PATH);
        assert!(reader.is_ok());

        assert!(reader.unwrap().filename().contains("BarOnAtTen_1.siff"));
    }

    #[test]
    fn test_siff_to_tiff() {
        let mut pb = PathBuf::from(TEST_FILE_PATH);
        pb.set_extension("tiff");
        assert!(!pb.exists());
        let result = siff_to_tiff(TEST_FILE_PATH, TiffMode::ScanImage, None);
        assert!(result.is_ok());
        assert!(pb.exists());

        use std::fs;
        fs::remove_file(pb).expect("Failed to remove test file");
    }

    // TiffMode tests
    #[test]
    fn test_tiff_mode_from_string_slice() {
        let mode = TiffMode::from_string_slice("OME");
        assert!(mode.is_ok_and(|val| val == TiffMode::OME));

        let mode = TiffMode::from_string_slice("ScanImage");
        assert!(mode.is_ok_and(|val| val == TiffMode::ScanImage));

        let mode = TiffMode::from_string_slice("Invalid");
        assert!(mode.is_err());
    }
}

#[no_mangle]
pub extern fn test_extern(){
    println!("Hello from Rust!");
}

/// `open_siff_extern(filename, len)` opens a `.siff` file
/// or a ScanImage-Flim `.tiff` file and returns a functional
/// `SiffReader` object pointer -- with responsibility for memory
/// management on the part of the caller.
/// 
/// ## Arguments
/// 
/// * `filename` - A pointer to a `u8` array that holds the name of the file to open
/// * `len` - The length of the `filename` array
/// 
/// ## Example
/// 
/// ```c
/// char *filename = "file.siff";
/// void* siffreader = open_siff_extern(filename, strlen(filename));
/// /*
/// ...
/// do stuff with siffreader
/// read some frames, who knows?
/// ...
/// */
/// 
/// close_siff_extern(siffreader);
/// ```
#[no_mangle]
pub extern "C" fn open_siff_extern(filename : *const u8, len : usize) -> *mut SiffReader {
    let filename = unsafe {
        std::slice::from_raw_parts(filename, len-1)
    };
    let filename = std::str::from_utf8(filename).unwrap();
    let reader = open_siff(filename).unwrap();
    Box::into_raw(Box::new(reader))
}

/// `close_siff_extern(reader)` frees the memory allocated for the `SiffReader`
/// object in lieu of freeing the memory in `C`.
/// 
/// ## Arguments
/// 
/// * `reader` - A pointer to the `SiffReader` object
/// 
/// ## Example
/// 
/// ```c
/// void *siffreader;
/// close_siff_extern(siffreader);
/// ```
/// 
#[no_mangle]
pub extern "C" fn close_siff_extern(reader : *mut SiffReader) -> () {
    let _ = unsafe {
        assert!(!reader.is_null());
        Box::from_raw(reader)
    };
}

/// NOT IMPLEMENTED
#[no_mangle]
pub extern "C" fn get_frames_extern(
    reader : *mut SiffReader,
    frames : *const u64,
    len : usize) -> i32 {
    
    let reader = unsafe {
        assert!(!reader.is_null());
        &*reader
    };
    let frames = unsafe {
        std::slice::from_raw_parts(frames, len)
    };
    let _frames = frames.to_vec();
    reader.filename();
    0
    //0;
    //reader.get_histogram();
    //let frames_array = reader.get_frames_intensity(&frames, None);
    //frames_array.unwrap().sum() as i32
}
