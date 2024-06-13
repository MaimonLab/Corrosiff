/// The primary `SiffReader` object, which
/// parses files and extracts interesting
/// information and/or data.
use std::{
    fs::File,
    fmt::Display,
    io::Result as IOResult,
    collections::HashMap,
    path::{Path, PathBuf},
};

use binrw::io::BufReader;
use ndarray::prelude::*;
use rayon::prelude::*;

// my module structure is probably
// too complex
use crate::{
    tiff::{
        FileFormat,
        BigTiffIFD,
        IFD,
    },
    data::image::{
        load_array_intensity,
        load_array_intensity_registered,
        load_histogram,
        DimensionsError,
        Dimensions,
    },
    metadata::Metadata,
};

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

// Boilerplate frame checking code.

/// Iterates through all frames and check that there's a corresponding
/// IFD for each frame.
fn _check_frames_in_bounds(frames : &[u64], ifds : &Vec<BigTiffIFD>)
    -> Result<(), DimensionsError> {
    frames.iter().all(|&x| x < ifds.len() as u64)
        .then(||()).ok_or(DimensionsError::IncorrectFrames)
}

/// Checks whether all requested frames share a shape. If not
/// returns `None`, otherwise returns the shared shape.
fn _check_shared_shape(frames : &[u64], ifds : &Vec<BigTiffIFD>)
    -> Option<Dimensions> {
    let array_dims = ifds[frames[0] as usize].dimensions().unwrap();
    frames.iter().all(
        |&x| {
            ifds[x as usize].dimensions().unwrap() == array_dims
        })
    .then(||array_dims)
}

/// Returns `Ok` if every element of `frames` is a key of `registration`.
fn _check_registration(registration : &Option<&HashMap<u64, (i32, i32)>>, frames : &[u64])
    -> Result<(), FramesError> {
    if let Some(reg) = registration {
        reg.iter().all(|(k, _)| frames.contains(k))
            .then(||()).ok_or(FramesError::RegistrationFramesMissing)
    } else {
        Ok(())
    }
}

/// `parallelize_array_op!(array, chunk_size, frames, op)`
/// 
/// Divides the array into chunks and parallelizes the operation
/// `op` on each chunk. The operation `op` should take a slice of
/// frames, a mutable reference to a chunk of the array along its 0th axis,
/// and able to accept a `std::fs::File`, with the signature 
/// `op(frames : &[u64], chunk : &mut ArrayBase, reader : &mut Reader)`. Opens
/// local copies of the file for reading.
macro_rules! parallelize_array_op{
    ($array : ident, $chunk_size : literal, $frames : ident, $filename : expr, $op : expr) => {
        let n_threads = $frames.len()/$chunk_size + 1;
        let remainder = $frames.len() % n_threads;

        // Compute the bounds for each threads operation
        let mut offsets = vec![];
        let mut start = 0;
        for i in 0..n_threads {
            let end = start + $chunk_size + if i < remainder { 1 } else { 0 };
            offsets.push((start, end));
            start = end;
        }

        // Create an array of chunks to parallelize
        let array_chunks : Vec<_> = $array.axis_chunks_iter_mut(Axis(0), $chunk_size).collect();

        array_chunks.into_par_iter().enumerate().try_for_each(
            |(chunk_idx, mut chunk)| -> Result<(), FramesError> {
            // Get the frame numbers and ifds for the frames in the chunk
            let start = chunk_idx * $chunk_size;
            let end = ((chunk_idx + 1) * $chunk_size).min($frames.len());

            let local_frames = &$frames[start..end];
            let mut local_f = File::open(&$filename).unwrap();

            $op(local_frames, &mut chunk, &mut local_f)
            }
        )?;
    }
}

/// A struct for reading a `.siff` file
/// or a ScanImage-Flim `.tiff` file.
/// Has methods which return arrays of
/// image or FLIM
pub struct SiffReader {
    _file : File,
    _filename : PathBuf,
    file_format : FileFormat,
    _ifds : Vec<BigTiffIFD>,
    _image_dims : Option<Dimensions>,
}

impl SiffReader{
    
    /// Opens a file and returns a `SiffReader` object
    /// for interacting with the data if successful.
    /// 
    /// ## Arguments
    /// 
    /// * `filename` - A string slice that holds the name of the file to open
    /// 
    /// ## Example
    /// 
    /// ```rust, ignore
    /// let reader = SiffReader::open("file.siff");
    /// ```
    /// 
    /// ## Errors
    /// 
    /// * `std::io::Error` - If there is an error opening the file,
    /// it will be returned directly.
    /// 
    pub fn open<P : AsRef<Path>>(filename : P) -> IOResult<Self> {

        // Open the file and parse its formatting info
        let file = File::open(&filename)?;
        let mut buff = BufReader::new(&file);
        let file_format = {
            FileFormat::parse_filetype(&mut buff)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
        }?;

        // A small buffer for reading the IFDs which are quite small, using a smaller
        // buffer makes this run much faster
        let mut wee_buff = BufReader::with_capacity(400, &file);
        Ok(
            SiffReader {
            _filename : filename.as_ref().to_path_buf(),
            _image_dims : None,
            _ifds : file_format.get_ifd_iter(&mut wee_buff).collect(),
            file_format,
            _file : file,
            }
        )
    }

    /// Returns number of frames in the file
    /// (including flyback etc).
    pub fn num_frames(&self) -> usize {
        self._ifds.len()
    }

    /// Get value of the filename
    /// 
    /// # Example
    /// 
    /// ```rust, ignore
    /// let reader = SiffReader::open("file.siff");
    /// println!("{}", reader.filename());
    /// ```
    pub fn filename(&self) -> String {
        self._filename.to_str().unwrap().to_string()
    }

    /// Return the non-varying frame data, which
    /// contains all of the instrument settings
    pub fn nvfd(&self) -> String {
        self.file_format.nvfd.clone()
    }

    /// Return the mROI information
    pub fn roi_string(&self) -> String {
        self.file_format.roi_string.clone()
    }
    
    /// Return an array corresponding to the intensity
    /// data of the frames requested. The returned array
    /// is a 3D array, with the first dimension corresponding
    /// to the frame number, and the second and third dimensions
    /// corresponding to the `y` and `x` dimensions of the frame.
    /// 
    /// ## Arguments
    /// 
    /// * `frames` - A slice of `u64` values corresponding to the
    /// frame numbers to retrieve
    /// 
    /// * `registration` - An optional `HashMap<u64, (i32, i32)>` which
    /// contains the pixel shifts for each frame. If this is `None`,
    /// the frames are read unregistered (marginally faster).
    /// 
    /// ## Example
    /// 
    /// ```rust, ignore
    /// let reader = SiffReader::open("file.siff");
    /// let frames = reader.get_frames_intensity(
    ///     &[0, 1, 2],
    ///     None
    /// );
    /// ```
    /// 
    /// ## Errors
    /// 
    /// * `FramesError::DimensionsError(DimensionsError)` - If the frames requested
    /// are out of bounds or do not all share the same shape (the underlying
    /// `DimensionsError` is attached to this error)
    /// 
    /// * `FramesError::IOError(_)` - If there is an error reading the file (with
    /// the underlying error attached)
    /// 
    /// * `FramesError::RegistrationFramesMissing` - If registration is used, and
    /// the registration values are missing for some frames
    /// 
    /// ## Returns
    /// 
    /// * `Result<Array3<u16>, FramesError>` - A 3D array of `u16` values
    /// corresponding to the intensity data of the frames requested.
    pub fn get_frames_intensity(
        &self,
        frames : &[u64],
        registration : Option<&HashMap<u64, (i32, i32)>>,
    ) -> Result<Array3<u16>, FramesError> { 
        
        // Check that the frames are in bounds
        _check_frames_in_bounds(&frames, &self._ifds).map_err(
                FramesError::DimensionsError)?;
        
        // Check that the frames share a shape
        let array_dims = self._image_dims.clone().or_else(
            || _check_shared_shape(frames, &self._ifds)
        ).ok_or(FramesError::DimensionsError(
            DimensionsError::NoConsistentDimensions)
        )?;

        // Check that every frame requested has a registration value,
        // if registration is used. Otherwise just ignore.
        _check_registration(&registration, &frames)?;

        // Create the array
        let mut array = Array3::<u16>::zeros((frames.len(), array_dims.ydim as usize, array_dims.xdim as usize));

        parallelize_array_op!(
            array, 
            5000, 
            frames, 
            self._filename,
            |frames : &[u64], chunk : &mut ArrayBase<_, Ix3>, reader : &mut File| {
                match registration {
                    Some(reg) => {
                        frames.iter().zip(chunk.axis_iter_mut(Axis(0)))
                            .try_for_each(
                                |(&this_frame, mut this_chunk)|
                                -> Result<(), FramesError> {
                                load_array_intensity_registered(
                                    reader,
                                    &self._ifds[this_frame as usize],
                                    &mut this_chunk,
                                    *reg.get(&this_frame).unwrap(),
                                ).map_err(FramesError::IOError)
                            })?;
                    },
                    None => {
                        frames.iter().zip(chunk.axis_iter_mut(Axis(0)))
                            .try_for_each(
                                |(&this_frame, mut this_chunk)|
                                -> Result<(), FramesError> {
                                load_array_intensity(
                                    reader,
                                    &self._ifds[this_frame as usize],
                                    &mut this_chunk,
                                ).map_err(FramesError::IOError)
                            })?;
                    },
                }
                Ok(())
            }
        );
        //)

        // // One thread handles at most 5k frames
        // let n_threads = frames.len()/5000 + 1;        
        // let chunk_size = frames.len() / n_threads;
        // let remainder = frames.len() % n_threads;

        // let mut offsets = vec![];
        // let mut start = 0;
        // for i in 0..n_threads {
        //     let end = start + chunk_size + if i < remainder { 1 } else { 0 };
        //     offsets.push((start, end));
        //     start = end;
        // }

        // let array_chunks : Vec<_> = array.axis_chunks_iter_mut(Axis(0), chunk_size).collect();

        // array_chunks.into_par_iter().enumerate().try_for_each(
        //     |(chunk_idx, mut chunk)| -> Result<(), FramesError> {
        //     // Get the frame numbers and ifds for the frames in the chunk
        //     let start = chunk_idx * chunk_size;
        //     let end = ((chunk_idx + 1) * chunk_size).min(frames.len());

        //     let local_frames = &frames[start..end];
        //     let local_ifds: Vec<&BigTiffIFD> = local_frames.iter().map(|&x| &self._ifds[x as usize]).collect();
            
        //     // A local reader of the file -- each reader needs to be independent
        //     let mut local_f = File::open(&self._filename).unwrap();
            
        //     match registration {
        //         Some(reg) => {
        //             local_ifds.iter().zip(local_frames.iter()).enumerate()
        //                 .try_for_each(
        //                     |(local_idx, (&this_ifd, this_frame))|
        //                     -> Result<(), FramesError> {
        //                     // Call `load_array_intensity_registered` on each chunk
        //                     load_array_intensity_registered(
        //                         &mut local_f,
        //                         this_ifd,
        //                         &mut chunk.index_axis_mut(Axis(0), local_idx),
        //                         *reg.get(this_frame).unwrap(),
        //                     ).map_err(FramesError::IOError)
        //                 })?;
        //         },
        //         None => {
        //             local_ifds.iter().enumerate()
        //                 .try_for_each(
        //                     // Call `load_array_intensity` on each chunk
        //                     |(local_idx, &this_ifd)|
        //                     -> Result<(), FramesError> {
        //                     load_array_intensity(
        //                         &mut local_f,
        //                         this_ifd,
        //                         &mut chunk.index_axis_mut(Axis(0), local_idx),
        //                     ).map_err(FramesError::IOError)
        //                 })?;
        //         },
        //     }
        //     Ok(())
        // })?;
        Ok(array)
    }

    /// Returns a 2D array of `u64` values corresponding to the
    /// number of photons in each bin of the arrival time histogram
    /// summing across ALL photons in the frames requested (not masked!).
    /// 
    /// ## Arguments
    /// 
    /// * `frames` - A slice of `u64` values corresponding to the
    /// frame numbers to retrieve
    /// 
    /// ## Returns
    /// 
    /// * `histogram` - An `ndarray::Array2<u64>` with the first dimension
    /// equal to the number of frames and the second dimension equal to the
    /// number of arrival time bins of the histogram (read from the `.siff`
    /// metadata).
    /// 
    /// ## Example
    /// 
    /// ```rust, ignore
    /// let reader = SiffReader::open("file.siff");
    /// let hist = reader.get_histogram(&[0, 1, 2]);
    /// 
    /// assert_eq!(reader.metadata().picoseconds_per_bin(), 20)
    /// 
    /// // 629 bins * 20 picoseconds per bin = 12.58 nanoseconds, ~80 MHz
    /// assert_eq!(hist.shape(), &[3, 629]);
    /// ```
    /// 
    pub fn get_histogram(&self, frames : &[u64]) -> Result<Array2<u64>, FramesError> {

        _check_frames_in_bounds(frames, &self._ifds).map_err(|err| FramesError::DimensionsError(err))?;

        let mut array = Array2::<u64>::zeros(
            (
                frames.len(),
                self.file_format.num_flim_tau_bins()
                .ok_or(FramesError::FormatError("Could not compute tau bins for file".to_string()))? as usize
            )
        );

        let n_threads = frames.len()/5000 + 1;
        let chunk_size = frames.len() / n_threads;
        let remainder = frames.len() % n_threads;

        let mut offsets = vec![];
        let mut start = 0;
        for i in 0..n_threads {
            let end = start + chunk_size + if i < remainder { 1 } else { 0 };
            offsets.push((start, end));
            start = end;
        }

        let array_chunks : Vec<_> = array.axis_chunks_iter_mut(Axis(0), chunk_size).collect();

        array_chunks.into_par_iter().enumerate().try_for_each(
            |(chunk_idx, mut chunk)| -> Result<(), FramesError> {
            // Get the frame numbers and ifds for the frames in the chunk
            let start = chunk_idx * chunk_size;
            let end = ((chunk_idx + 1) * chunk_size).min(frames.len());

            let local_frames = &frames[start..end];
            let local_ifds: Vec<&BigTiffIFD> = local_frames.iter().map(|&x| &self._ifds[x as usize]).collect();

            let mut local_f = File::open(&self._filename).unwrap();
            
            local_ifds.iter().enumerate()
                .try_for_each(
                    |(local_idx, &this_ifd)|
                    -> Result<(), FramesError> {
                    load_histogram(
                        this_ifd,
                        &mut local_f,
                        &mut chunk.index_axis_mut(Axis(0), local_idx),
                    ).map_err(FramesError::IOError)
                    }
                )?;
            Ok(())
            })?;

        // for (idx, &frame) in frames.iter().enumerate() {
        //     let ifd = &self._ifds[frame as usize];
        //     let mut f = File::open(&self._filename).unwrap();
        //     load_histogram(ifd, &mut f, &mut array.slice_mut(s![idx, ..]))
        //     .map_err(|e| FramesError::IOError(e))?;
        // }

        Ok(array)
    }

    pub fn get_histogram_mask(&self, frames : &[u64], mask : &[u8]) {
        unimplemented!()
    }

}

impl Display for SiffReader {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "SiffReader: {}\n{} frames",
            self._filename.to_str().unwrap(),
            self._ifds.len(),
            //self._ifd_pointers.len(),
        )
    }
}

// impl Iterator for SiffReader {
//     type Item = usize;
//     fn next(&mut self) -> Option<Self::Item> {
//         self._ifds.pop()
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TEST_FILE_PATH;

    #[test]
    fn test_open_siff() {
        let reader = SiffReader::open(TEST_FILE_PATH);
        assert!(reader.is_ok());
    }

    #[test]
    fn read_frame() {
        let reader = SiffReader::open(TEST_FILE_PATH).unwrap();
        // Compressed frame
        let frame = reader.get_frames_intensity(&[35], None);
        assert!(frame.is_ok(), "Error: {:?}", frame);
        assert_eq!(frame.unwrap().sum(), 63333);

        let mut reg: HashMap<u64, (i32, i32)> = HashMap::new();
        reg.insert(32, (0, 0));
        // Compressed frame with registration
        let frame = reader.get_frames_intensity(&[35], Some(&reg));
        assert!(frame.is_err());
        reg.insert(35, (0, 0));
        let frame = reader.get_frames_intensity(&[35], Some(&reg));
        if frame.is_err() {
            println!("{:?}", frame);
        }
        assert!(frame.is_ok());
        assert_eq!(frame.unwrap().sum(), 63333);

        // Uncompressed frame
        let frame = reader.get_frames_intensity(&[15], None);
        assert!(frame.is_ok());
        assert_eq!(frame.unwrap().sum(), 794);
    }

    use rand::Rng;
    /// Read several frames and test.
    #[test]
    fn read_frames(){
        let reader = SiffReader::open(TEST_FILE_PATH).unwrap();
        let frames = reader.get_frames_intensity(&[15, 35, 35], None);
        assert!(frames.is_ok());
        let frames = frames.unwrap();
        assert_eq!(frames.index_axis(Axis(0),0).sum(), 794);
        assert_eq!(frames.index_axis(Axis(0),1).sum(), 63333);
        assert_eq!(frames.index_axis(Axis(0),2).sum(), 63333);

        let mut frame_vec = vec![35; 40000];
        frame_vec[22] = 15;

        let mut reg = HashMap::<u64, (i32, i32)>::new();
        reg.insert(15, (-5, 10));
        reg.insert(35, (0, 1));

        let frames = reader.get_frames_intensity(&frame_vec, None);

        assert!(frames.is_ok());
        let frames = frames.unwrap();

        assert_eq!(frames.index_axis(Axis(0),22).sum(), 794);

        let mut rng = rand::thread_rng();
        for _ in 0..400 {
            // spot check -- they should all be the same but this makes sure no random
            // elements are wrong.
            assert_eq!(frames.index_axis(Axis(0), rng.gen_range(0..40000)).sum(), 63333);
        }
    }

    #[test]
    fn read_histogram() {
        let reader = SiffReader::open(TEST_FILE_PATH).unwrap();
        
        let framelist = vec![15];
        let hist = reader.get_histogram(&framelist);
        assert!(hist.is_ok());
        let hist = hist.unwrap();
        // More and more mysterious
        let frames = reader.get_frames_intensity(&framelist, None);
        assert_eq!(hist.sum(), frames.unwrap().fold(0 as u64, |sum, &x| sum + (x as u64)));
    }
}