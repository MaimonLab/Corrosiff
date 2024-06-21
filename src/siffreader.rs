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
use itertools::izip;
use ndarray::prelude::*;
use rayon::prelude::*;

// my module structure is probably
// too complex
use crate::{
    utils::{
        parallelize_op,
        FramesError,
    },
    CorrosiffError,
    tiff::{
        FileFormat,
        BigTiffIFD,
        IFD,
        dimensions_consistent,
    },
    data::image::{
        load_array_intensity,
        load_array_intensity_registered,
        sum_intensity_mask,
        sum_intensity_mask_registered,
        load_flim_empirical_and_intensity_arrays,
        load_flim_empirical_and_intensity_arrays_registered,
        load_histogram,
        DimensionsError,
        Dimensions,
        roll,
    },
    metadata::{
        FrameMetadata,
        get_experiment_timestamps,
        get_epoch_timestamps_laser,
        get_epoch_timestamps_system,
        get_epoch_timestamps_both,
        get_appended_text,
    }
};

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
/// If `registration` is totally empty, converts it to `None` and returns `Ok`.
/// If it's only partially populated, returns an error.
fn _check_registration(registration : &mut Option<&HashMap<u64, (i32, i32)>>, frames : &[u64])
    -> Result<(), FramesError> {
    if let Some(reg) = registration {
        if reg.is_empty() {
            *registration = None;
            Ok(())
        } else {
            frames.iter().all(|k| reg.contains_key(k))
            .then(||()).ok_or(FramesError::RegistrationFramesMissing)
        }
    } else {
        Ok(())
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
        let _ifds = file_format.get_ifd_iter(&mut wee_buff).collect::<Vec<_>>();
        Ok(
            SiffReader {
            _filename : filename.as_ref().to_path_buf(),
            _image_dims : dimensions_consistent(&_ifds),
            _ifds,
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

    pub fn frames_vec(&self) -> Vec<u64> {
        (0..self.num_frames() as u64).collect()
    }

    pub fn image_dims(&self) -> Option<Dimensions> {
        self._image_dims.clone()
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

    /// Return the mROI information as a string
    pub fn roi_string(&self) -> String {
        self.file_format.roi_string.clone()
    }

    /// Returns whether the file uses the BigTIFF
    /// format or the standard 32-bit TIFF format.
    pub fn is_bigtiff(&self) -> bool {
        self.file_format.is_bigtiff()
    }

    /// Returns whether the file being read is a
    /// .siff (and contains lifetime information)
    /// or just a regular tiff file.
    /// 
    /// For now, this is implemented is a very
    /// unsophisticated manner -- it simply
    /// checks whether the file ends in `.siff`!
    /// 
    /// TODO: Better!
    pub fn is_siff(&self) -> bool {
        self._filename.to_str().unwrap().ends_with(".siff")
    }

    /// Return the metadata objects corresponding to
    /// each of the requested frames.
    pub fn get_frame_metadata(&self, frames : &[u64]) 
        -> Result<Vec<FrameMetadata>, CorrosiffError> {

            _check_frames_in_bounds(frames, &self._ifds).map_err(
            |err| FramesError::DimensionsError(err)
        )?;

        let mut metadata = Vec::with_capacity(frames.len());
        let mut f = File::open(&self._filename).unwrap();
        for frame in frames {
            metadata.push(FrameMetadata::from_ifd_and_file(
                &self._ifds[*frame as usize],
                &mut f
            )?);
        }
        Ok(metadata)
    }

    /// Return an array of timestamps corresponding to the
    /// experiment time of each frame requested (seconds since
    /// the onset of the acquisition).
    /// 
    /// ## Arguments
    /// 
    /// * `frames` - A slice of `u64` values corresponding to the
    /// frame numbers to retrieve
    /// 
    /// ## Returns
    /// 
    /// * `Result<Array1<f64>, CorrosiffError>` - An array of `f64` values
    /// corresponding to the timestamps of the frames requested (in units
    /// of seconds since beginning of the acquisition).
    /// 
    /// ## Example
    /// 
    /// ```rust, ignore
    /// let reader = SiffReader::open("file.siff");
    /// let timestamps = reader.get_experiment_timestamps(&[0, 1, 2]);
    /// ```
    pub fn get_experiment_timestamps(
        &self, frames : &[u64]
    ) -> Result<Array1<f64>, CorrosiffError> {

        _check_frames_in_bounds(frames, &self._ifds)?;
        let mut array = Array1::<f64>::zeros(frames.len());

        parallelize_op!(
            array, 
            5000, 
            frames, 
            self._filename.to_str().unwrap(),
            | filename : &str |{BufReader::with_capacity(800, File::open(filename).unwrap())},
            |frames : &[u64], chunk : &mut ArrayBase<_, Ix1>, reader : &mut BufReader<File>| {
                let ifds = frames.iter().map(|&x| &self._ifds[x as usize]).collect::<Vec<_>>();
                get_experiment_timestamps(&ifds, reader)
                    .iter().zip(chunk.iter_mut())
                    .for_each(|(&x, y)| *y = x);
                Ok(())
            }
        );
        Ok(array)    
    }

    /// Return an array of timestamps corresponding to the
    /// epoch time of each frame requested computed using
    /// the number of laser pulses into the acquisition at the
    /// time of the frame trigger (and the estimated pulse rate).
    /// 
    /// This measurement has extremely low jitter but a fixed
    /// rate of drift from "true" epoch (as the system clock might
    /// read it).
    /// 
    /// ## Arguments
    /// 
    /// * `frames` - A slice of `u64` values corresponding to the
    /// frame numbers to retrieve
    /// 
    /// ## Returns
    /// 
    /// * `Result<Array1<u64>, CorrosiffError>` - An array of `u64` values
    /// corresponding to the epoch time of each frame
    /// 
    /// ## Example
    /// 
    /// ```rust, ignore
    /// let reader = SiffReader::open("file.siff");
    /// let timestamps = reader.get_epoch_timestamps(&[0, 1, 2]);
    /// ```
    pub fn get_epoch_timestamps_laser(
        &self, frames : &[u64]
    ) -> Result<Array1<u64>, CorrosiffError> {
        _check_frames_in_bounds(frames, &self._ifds)?;

        let mut array = Array1::<u64>::zeros(frames.len());

        parallelize_op!(
            array, 
            5000, 
            frames, 
            self._filename.to_str().unwrap(),
            | filename : &str |{BufReader::with_capacity(800, File::open(filename).unwrap())},
            |frames : &[u64], chunk : &mut ArrayBase<_, Ix1>, reader : &mut BufReader<File>| {
                let ifds = frames.iter().map(|&x| &self._ifds[x as usize]).collect::<Vec<_>>();
                get_epoch_timestamps_laser(&ifds, reader)
                    .iter().zip(chunk.iter_mut())
                    .for_each(|(&x, y)| *y = x);
                Ok(())
            }
        );
        Ok(array)
    }

    /// Return an array of timestamps corresponding to the
    /// most recent epoch timestamped system call at the time
    /// of the frame trigger. This is the most accurate measure
    /// of system time because it does not drift, but the system
    /// is only queried about once a second, so there is high
    /// _apparent jitter_, with many consecutive frames sharing
    /// a value.
    /// 
    /// ## Arguments
    /// 
    /// * `frames` - A slice of `u64` values corresponding to the
    /// frame numbers to retrieve
    /// 
    /// ## Returns
    /// 
    /// * `Result<Array1<u64>, CorrosiffError>` - An array of `u64` values
    /// corresponding to the system time of each frame
    /// 
    /// ## Example
    /// 
    /// ```rust, ignore
    /// let reader = SiffReader::open("file.siff");
    /// reader.get_epoch_timestamps_system(&[0, 1, 2]);
    /// ```
    /// 
    /// ## Errors
    /// 
    /// * `CorrosiffError::NoSystemTimestamps` - If the system timestamps
    /// are not present in the file, this error is returned.
    pub fn get_epoch_timestamps_system(
        &self, frames : &[u64]
    ) -> Result<Array1<u64>, CorrosiffError> {
        _check_frames_in_bounds(frames, &self._ifds)?;

        let mut array = Array1::<u64>::zeros(frames.len());

        let op = 
        | frames : &[u64], chunk : &mut ArrayViewMut1<u64>, reader : &mut BufReader<File> |
        -> Result<(), CorrosiffError> {
            let ifds = frames.iter().map(|&x| &self._ifds[x as usize]).collect::<Vec<_>>();
            get_epoch_timestamps_system(&ifds, reader)?
                .iter().zip(chunk.iter_mut())
                .for_each(|(&x, y)| *y = x.unwrap());
            Ok(())
        };

        parallelize_op!(
            array, 
            5000, 
            frames, 
            self._filename.to_str().unwrap(),
            | filename : &str | { BufReader::with_capacity(800, File::open(filename).unwrap()) },
            op
        );

        Ok(array)
    }

    /// Return an array of timestamps corresponding to the
    /// two measurements of epoch time in the data: the
    /// laser clock synched one (*low jitter, some drift*)
    /// and the system call one (*high jitter, no drift*).
    /// 
    /// The two can be combined to allow much more reliable
    /// estimation of the timestamp of every frame trigger
    /// in absolute epoch time determined by the PTP system.
    /// These data are in nanoseconds since epoch.
    /// 
    /// ## Arguments
    /// 
    /// * `frames` - A slice of `u64` values corresponding to the
    /// frame numbers to retrieve
    /// 
    /// ## Returns
    /// 
    /// * `Result<Array2<u64>, CorrosiffError>` - A 2D array of `u64` values
    /// corresponding to the two epoch timestamps of each frame. The first
    /// row is `laser_clock` values, the second row is `system_clock` values.
    /// The `system_clock` changes only once a second or so, but this is much
    /// faster than the drift of the laser clock.
    /// 
    /// ## Example
    /// 
    /// ```rust, ignore
    /// let reader = SiffReader::open("file.siff");
    /// reader.get_epoch_timestamps_both(&[0, 1, 2]);
    /// ```
    /// 
    /// ## Errors
    /// 
    /// * `CorrosiffError::NoSystemTimestamps` - If the system timestamps
    /// are not present in the file, this error is returned.
    /// 
    /// * `CorrosiffError::DimensionsError(DimensionsError)` - If the frames requested
    /// are out of bounds (the underlying `DimensionsError` is attached to this error)
    pub fn get_epoch_timestamps_both(
        &self, frames : &[u64]
    ) -> Result<Array2<u64>, CorrosiffError> {
        _check_frames_in_bounds(frames, &self._ifds)?;

        let mut array = Array2::<u64>::zeros((2, frames.len()));

        let chunk_size = 5000;
        let n_threads = frames.len()/chunk_size + 1;
        let remainder = frames.len() % n_threads;

        // Compute the bounds for each threads operation
        let mut offsets = vec![];
        let mut start = 0;
        for i in 0..n_threads {
            let end = start + chunk_size + if i < remainder { 1 } else { 0 };
            offsets.push((start, end));
            start = end;
        }

        // Create an array of chunks to parallelize
        let array_chunks : Vec<_> = array.axis_chunks_iter_mut(Axis(1), chunk_size).collect();

        array_chunks.into_par_iter().enumerate().try_for_each(
            |(chunk_idx, mut chunk)| -> Result<(), CorrosiffError> {
            // Get the frame numbers and ifds for the frames in the chunk
            let start = chunk_idx * chunk_size;
            let end = ((chunk_idx + 1) * chunk_size).min(frames.len());

            let local_frames = &frames[start..end];
            let mut local_f = BufReader::with_capacity(800, 
                File::open(&self._filename).unwrap()
            );

            let ifds = local_frames.iter().map(|&x| &self._ifds[x as usize]).collect::<Vec<_>>();
            get_epoch_timestamps_both(&ifds, &mut local_f)?
                .iter().zip(chunk.axis_iter_mut(Axis(1)))
                .for_each(|(x, mut y)|{
                y[0] = x.0; y[1] = x.1}
            );
            Ok(())
            }
        )?;
        Ok(array)
    }

    /// Returns a vector of all frames containing appended text.
    /// Each element of the vector is a tuple containing the frame
    /// number, the text itself, and the timestamp of the frame (if present).
    /// 
    /// ## Returns
    /// 
    /// (`frame_number`, `text`, Option<`timestamp`>)
    pub fn get_appended_text(&self, frames : &[u64]) -> Vec<(u64, String, Option<f64>)> {
        let mut f = File::open(&self._filename).unwrap();
        let ifd_by_ref = frames.iter().map(|&x| &self._ifds[x as usize]).collect::<Vec<_>>();
        get_appended_text(&ifd_by_ref, &mut f)
        .iter().map(
            |(idx, this_str, this_timestamp)|
            (frames[*idx as usize], this_str.clone(), *this_timestamp)
        ).collect()
    }

    /******************************
     * 
     * Frame data methods
     * 
     * ***************************
     */

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
    /// the frames are read unregistered (runs faster).
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
    ) -> Result<Array3<u16>, CorrosiffError> { 
        
        // Check that the frames are in bounds
        _check_frames_in_bounds(&frames, &self._ifds).map_err(
                FramesError::DimensionsError)?;
        
        // Check that the frames share a shape
        let array_dims = self._image_dims.clone().or_else(
            || _check_shared_shape(frames, &self._ifds)
        ).ok_or(FramesError::DimensionsError(
            DimensionsError::NoConsistentDimensions)
        )?;

        let mut registration = registration;
        // Check that every frame requested has a registration value,
        // if registration is used. Otherwise just ignore.
        _check_registration(&mut registration, &frames)?;

        // Create the array
        let mut array = Array3::<u16>::zeros((frames.len(), array_dims.ydim as usize, array_dims.xdim as usize));

        let op = | frames : &[u64], chunk : &mut ArrayViewMut3<u16>, reader : &mut File |
        -> Result<(), CorrosiffError> {
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
        };
        
        parallelize_op!(
            array, 
            2500, 
            frames, 
            self._filename,
            op
        );
        
        Ok(array)
    }

    /// Return two arrays: the intensity (photon counts) and
    /// the empirical lifetime (in arrival time bins) of each
    /// pixel of the frames requested.
    /// corresponding to the `y` and `x` dimensions of the frame.
    /// The lifetime array is a 3D `f64` array with the first dimension
    /// corresponding to the frame number, and the second and third dimensions
    /// corresponding to the `y` and `x` dimensions of the frame.
    /// The intensity array is a 3D `u16` array with the same shape.
    /// If `registration` is `None`, the frames are read unregistered,
    /// otherwise they are registered in place.
    /// 
    /// ## Arguments
    /// 
    /// * `frames` - A slice of `u64` values corresponding to the
    /// frame numbers to retrieve
    /// 
    /// * `registration` - An optional `HashMap<u64, (i32, i32)>` which
    /// contains the pixel shifts for each frame. If this is `None`,
    /// the frames are read unregistered (runs faster).
    /// 
    /// ## Returns
    /// 
    /// * `Result<(Array3<f64>, Array3<u16>), CorrosiffError>` - A tuple
    /// containing the lifetime and intensity arrays of the frames requested
    /// (in that order). The lifetime array is the empirical lifetime in
    /// units of arrival time bins of the MultiHarp -- meaning for every frame,
    /// each pixel contains the average arrival time of all photons in that pixel
    /// during that frame.
    /// 
    /// ## Example
    /// 
    /// ```rust, ignore
    /// let reader = SiffReader::open("file.siff");
    /// let (lifetime, intensity) = reader.get_frames_flim(
    ///    &[0, 1, 2],
    ///   None
    /// );
    /// let intensity_alone = reader.get_frames(
    ///   &[0, 1, 2],
    ///  None
    /// );
    /// assert_eq!(intensity, intensity_alone);
    /// // intensity is a 3D array of u16 values
    /// // lifetime is a 3D array of f64 values in units of
    /// // arrival time bins of the MultiHarp.
    /// ```
    /// 
    /// ## Errors
    /// 
    /// * `CorrosiffError::DimensionsError(DimensionsError)` - If the frames requested
    /// are out of bounds or do not all share the same shape (the underlying
    /// `DimensionsError` is attached to this error)
    /// 
    /// * `CorrosiffError::FramesError(FramesError::IOError(_))` - If there is an error reading the file (with
    /// the underlying error attached)
    /// 
    /// * `CorrosiffError::FramesError(FramesError::RegistrationFramesMissing)` - If registration is used, and
    /// the registration values are missing for some frames
    /// 
    /// ## See also
    /// 
    /// - `get_frames_intensity` - for just the intensity data
    /// - `get_histogram` to pool all photons for a frame into a histogram
    /// 
    /// ## Panics
    /// 
    /// ???
    pub fn get_frames_flim(
        &self,
        frames : &[u64],
        registration : Option<&HashMap<u64, (i32, i32)>>,
    ) -> Result<(Array3<f64>, Array3<u16>), CorrosiffError> {
        
        // Check that the frames are in bounds
        _check_frames_in_bounds(&frames, &self._ifds).map_err(
            FramesError::DimensionsError)?;
        
        // Check that the frames share a shape
        let array_dims = self._image_dims.clone().or_else(
            || _check_shared_shape(frames, &self._ifds)
        ).ok_or(FramesError::DimensionsError(
            DimensionsError::NoConsistentDimensions)
        )?;

        let mut registration = registration;

        // Check that every frame requested has a registration value,
        // if registration is used. Otherwise just ignore.
        _check_registration(&mut registration, &frames)?;

        let (mut lifetime, mut intensity) = (
            Array3::<f64>::zeros((frames.len(), array_dims.ydim as usize, array_dims.xdim as usize)),
            Array3::<u16>::zeros((frames.len(), array_dims.ydim as usize, array_dims.xdim as usize))
        ); 

        let op = 
        |
            frames : &[u64],
            chunk_intensity : &mut ArrayViewMut3<u16>,
            chunk_lifetime : &mut ArrayViewMut3<f64>,
            reader : &mut File
        | -> Result<(), CorrosiffError> {
            match registration {
                Some(reg) => {
                    izip!(
                        frames,
                        chunk_lifetime.axis_iter_mut(Axis(0)),
                        chunk_intensity.axis_iter_mut(Axis(0))
                    ).try_for_each(
                        
                        |(&this_frame, mut this_chunk_l, mut this_chunk_i)|
                        -> Result<(), CorrosiffError> {
                        load_flim_empirical_and_intensity_arrays_registered(
                            reader,
                            &self._ifds[this_frame as usize],
                            &mut this_chunk_l,
                            &mut this_chunk_i,
                            *reg.get(&this_frame).unwrap(),
                        )
                        }

                    )?;
                },
                None => {
                    izip!(
                        frames,
                        chunk_lifetime.axis_iter_mut(Axis(0)),
                        chunk_intensity.axis_iter_mut(Axis(0))
                    ).try_for_each(

                        |(&this_frame, mut this_chunk_l, mut this_chunk_i)|
                        -> Result<(), CorrosiffError> {
                        load_flim_empirical_and_intensity_arrays(
                            reader,
                            &self._ifds[this_frame as usize],
                            &mut this_chunk_l,
                            &mut this_chunk_i,
                        )
                        }

                    )?;
                },
            }
            Ok(())
        };
        
        parallelize_op!(
            (intensity, lifetime), 
            2500, 
            frames, 
            self._filename,
            op
        );
        Ok((lifetime, intensity))
    }

    /// Returns a 1D array of `u64` values corresponding to the
    /// photon stream of the frames requested, rather than converting
    /// the data into an array of intensity (or arrival) values.
    /// 
    /// ## Arguments
    /// 
    /// * `frames` - A slice of `u64` values corresponding to the
    /// frame numbers to retrieve
    /// 
    /// * `registration` - An optional `HashMap<u64, (i32, i32)>` which
    /// 
    /// ## Panics
    /// 
    /// *It's not implemented yet!*
    pub fn get_photon_stream(
        &self,
        frames : &[u64],
        registration : Option<&HashMap<u64, (i32, i32)>>,
    ) -> Result<Array1<u64>, CorrosiffError> {
        unimplemented!()
    }

    /***************
     * 
     * ROI-like methods
     * 
     * ************
     */

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
    pub fn get_histogram(&self, frames : &[u64]) -> Result<Array2<u64>, CorrosiffError> {

        _check_frames_in_bounds(frames, &self._ifds).map_err(|err| FramesError::DimensionsError(err))?;

        let mut array = Array2::<u64>::zeros(
            (
                frames.len(),
                self.file_format.num_flim_tau_bins()
                .ok_or(FramesError::FormatError("Could not compute tau bins for file".to_string()))? as usize
            )
        );

        let op = |frames : &[u64], chunk : &mut ArrayViewMut2<u64>, reader : &mut File|
        -> Result<(), CorrosiffError> {
            frames.iter().zip(chunk.axis_iter_mut(Axis(0)))
                .try_for_each(
                    |(&this_frame, mut this_chunk)|
                    -> Result<(), std::io::Error> {
                    load_histogram(
                        &self._ifds[this_frame as usize],
                        reader,
                        &mut this_chunk,
                    )
                })?;
            Ok(())
        };

        parallelize_op!(
            array, 
            5000, 
            frames, 
            self._filename,
            op
        );

        Ok(array)
    }

    pub fn get_histogram_mask(&self, frames : &[u64], mask : &Array2<bool>)
    -> Result<Array2<u64>, CorrosiffError> {
        unimplemented!()
    }

    /// Sums the intensity of the frames requested within the
    /// region of interest (ROI) specified by the boolean array
    /// `roi`. The ROI should be the same shape as the frames' 
    /// `y` and `x` dimensions
    /// 
    /// ## Arguments
    /// 
    /// * `roi` - A 2D boolean array with the same shape as the frames'
    /// `y` and `x` dimensions. The ROI is a mask which will be used
    /// to sum the intensity of the frames requested.
    /// 
    /// * `frames` - A slice of `u64` values corresponding to the
    /// frame numbers to retrieve
    /// 
    /// * `registration` - An optional `HashMap<u64, (i32, i32)>` which
    /// contains the pixel shifts for each frame. If this is `None`,
    /// the frames are read unregistered (runs faster).
    /// 
    /// ## Returns
    /// 
    /// * `Result<Array1<u64>, CorrosiffError>` - An array of `u64` values
    /// corresponding to the sum of the intensity of the frames requested
    /// within the ROI specified.
    /// 
    /// ## Example
    /// 
    /// ```rust, ignore
    /// let reader = SiffReader::open("file.siff");
    /// let roi = Array2::<bool>::from_elem((512, 512), true);
    /// // Set the ROI to false in the middle
    /// roi.slice(s![200..300, 200..300]).fill(false);
    /// 
    /// let sum = reader.sum_roi_flat(&roi, &[0, 1, 2], None);
    /// ```
    /// 
    /// ## Errors
    /// 
    /// * `CorrosiffError::DimensionsError(DimensionsError)` - If the frames requested
    /// are out of bounds, do not share the same shape, or the ROI does not share
    /// the same shape as the frames (the underlying `DimensionsError` is attached to this error)
    /// 
    /// 
    /// 
    pub fn sum_roi_flat(
        &self,
        roi : &ArrayView2<bool>,
        frames : &[u64],
        registration : Option<&HashMap<u64, (i32, i32)>>
    ) -> Result<Array1<u64>, CorrosiffError> {
         // Check that the frames are in bounds
         _check_frames_in_bounds(&frames, &self._ifds).map_err(
            FramesError::DimensionsError)?;
        
        // Check that the frames share a shape with the mask
        let array_dims = self._image_dims.clone().or_else(
            || _check_shared_shape(frames, &self._ifds)
        ).ok_or(FramesError::DimensionsError(
            DimensionsError::NoConsistentDimensions)
        )?;

        if array_dims.to_tuple() != roi.dim() {
            return Err(FramesError::DimensionsError(
                DimensionsError::MismatchedDimensions{
                    required : array_dims,
                    requested : Dimensions::from_tuple(roi.dim()),
                }
            ).into());
        }

        let mut registration = registration;

        // Check that every frame requested has a registration value,
        // if registration is used. Otherwise just ignore.
        _check_registration(&mut registration, &frames)?;

        let mut array = Array1::<u64>::zeros(frames.len());

        let op = |frames : &[u64], chunk : &mut ArrayViewMut1<u64>, reader : &mut File| 
        -> Result<(), CorrosiffError> {
            match registration {
                Some(reg) => {
                    frames.iter().zip(chunk.iter_mut())
                        .try_for_each(
                            |(&this_frame, mut this_frame_sum)|
                            -> Result<(), CorrosiffError> {
                            sum_intensity_mask_registered(
                                reader,
                                &self._ifds[this_frame as usize],
                                &mut this_frame_sum,
                                &roi.view(),
                                *reg.get(&this_frame).unwrap(),
                            )?; Ok(())
                    })?;
                },
                None => {
                    frames.iter().zip(chunk.iter_mut())
                        .try_for_each(
                            |(&this_frame, mut this_frame_sum)|
                            -> Result<(), CorrosiffError> {
                            sum_intensity_mask(
                                reader,
                                &self._ifds[this_frame as usize],
                                &mut this_frame_sum,
                                &roi.view(),
                            )?; Ok(())
                        })?;
                },
            }
            Ok(())
        };
        
        parallelize_op!(
            array,
            2500,
            frames,
            self._filename,
            op
        );

        Ok(array)
    }

    /// Sums the intensity of the frames requested within the
    /// region of interest (ROI) specified by the boolean array
    /// `roi`. The ROI should have final two dimensions the same
    /// as the frames' `y` and `x` dimensions, and the first
    /// dimension will be looped over while cycling through frames
    /// (i.e. frame 1 will be masked by the first plane of the ROI,
    /// frame 2 by the second plane, etc).
    pub fn sum_roi_volume(
        &self,
        roi : &ArrayView3<bool>,
        frames : &[u64],
        registration : Option<&HashMap<u64, (i32, i32)>>
    ) -> Result<Array1<u64>, CorrosiffError>{

        // Check that the frames are in bounds
        _check_frames_in_bounds(&frames, &self._ifds).map_err(
            FramesError::DimensionsError)?;

        // Check that the frames share a shape with the mask
        let array_dims = self._image_dims.clone().or_else(
            || _check_shared_shape(frames, &self._ifds)
        ).ok_or(FramesError::DimensionsError(
            DimensionsError::NoConsistentDimensions)
        )?;

        if array_dims.to_tuple() != (roi.dim().1, roi.dim().2) {
            return Err(FramesError::DimensionsError(
                DimensionsError::MismatchedDimensions{
                    required : array_dims,
                    requested : Dimensions::from_tuple((roi.dim().1, roi.dim().2)),
                }
            ).into());
        }

        let mut registration = registration;

        // Check that every frame requested has a registration value,
        // if registration is used. Otherwise just ignore.
        _check_registration(&mut registration, &frames)?;

        let mut array = Array1::<u64>::zeros(frames.len());

        // SIGH my macro skills are not good enough for this job.
        let CHUNK_SIZE = 2500;

        let n_threads = frames.len()/CHUNK_SIZE + 1;
        let remainder = frames.len() % n_threads;

        // Compute the bounds for each threads operation
        let mut offsets = vec![];
        let mut start = 0;
        for i in 0..n_threads {
            let end = start + CHUNK_SIZE + if i < remainder { 1 } else { 0 };
            offsets.push((start, end));
            start = end;
        }

        // Create an array of chunks to parallelize
        let array_chunks : Vec<_> = array.axis_chunks_iter_mut(Axis(0), CHUNK_SIZE).collect();

        array_chunks.into_par_iter().enumerate().try_for_each(
            |(chunk_idx, mut chunk)| -> Result<(), CorrosiffError> {
            // Get the frame numbers and ifds for the frames in the chunk
            let start = chunk_idx * CHUNK_SIZE;
            let end = ((chunk_idx + 1) * CHUNK_SIZE).min(frames.len());

            let local_frames = &frames[start..end];
            let mut local_f = File::open(self._filename.clone()).unwrap();
            
            let roi_cycle = roi.axis_iter(Axis(0)).cycle();
            // roi_cycle needs to be incremented by the start value
            // modulo the length of the roi_cycle
            let roi_cycle = roi_cycle.skip(start % roi.dim().0);

            match registration {
                Some(reg) => {
                    izip!(local_frames.iter(),chunk.iter_mut(), roi_cycle)
                        .try_for_each(
                            |(&this_frame, mut this_frame_sum, roi_plane)|
                            -> Result<(), CorrosiffError> {
                            sum_intensity_mask_registered(
                                &mut local_f,
                                &self._ifds[this_frame as usize],
                                &mut this_frame_sum,
                                &roi_plane,
                                *reg.get(&this_frame).unwrap(),
                            )?; Ok(())
                    })?;
                },
                None => {
                    izip!(local_frames.iter(),chunk.iter_mut(), roi_cycle)
                        .try_for_each(
                            |(&this_frame, mut this_frame_sum, roi_plane)|
                            -> Result<(), CorrosiffError> {
                            sum_intensity_mask(
                                &mut local_f,
                                &self._ifds[this_frame as usize],
                                &mut this_frame_sum,
                                &roi_plane,
                            )?; Ok(())
                        })?;
                },
            }
            Ok(())
            }
        )?;

        // parallelize_op![
        //     array,
        //     2500,
        //     frames,
        //     self._filename,
        //     op
        // ];
        
        Ok(array)

    }

    pub fn sum_rois_flat(
        &self,
        roi : &ArrayView3<bool>,
        frames : &[u64],
        registration : Option<&HashMap<u64, (i32, i32)>>
    ) -> Result<Array2<u64>, CorrosiffError> {
        unimplemented!()
    }

    pub fn sum_rois_volume(
        &self,
        roi : &ArrayView4<bool>,
        frames : &[u64],
        registration : Option<&HashMap<u64, (i32, i32)>>
    ) -> Result<Array2<u64>, CorrosiffError> {
        unimplemented!()
    }

    pub fn sum_rois_flim_flat(
        &self,
        roi : &ArrayView3<bool>,
        frames : &[u64],
        registration : Option<&HashMap<u64, (i32, i32)>>
    ) -> Result<(Array2<u64>, Array2<u64>), CorrosiffError> {
        unimplemented!()
    }

    pub fn sum_rois_flim_volume(
        &self,
        roi : &ArrayView4<bool>,
        frames : &[u64],
        registration : Option<&HashMap<u64, (i32, i32)>>
    ) -> Result<(Array2<u64>, Array2<u64>), CorrosiffError> {
        unimplemented!()
    }

    pub fn sum_rois_flim_flat_masked(
        &self,
        roi : &ArrayView3<bool>,
        frames : &[u64],
        registration : Option<&HashMap<u64, (i32, i32)>>
    ) -> Result<(Array2<u64>, Array2<u64>), CorrosiffError> {
        unimplemented!()
    }

    pub fn sum_rois_flim_volume_masked(
        &self,
        roi : &ArrayView4<bool>,
        frames : &[u64],
        registration : Option<&HashMap<u64, (i32, i32)>>
    ) -> Result<(Array2<u64>, Array2<u64>), CorrosiffError> {
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
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{data::image, tests::{BIG_FILE_PATH, COMPRESSED_FRAME_NUM, TEST_FILE_PATH, UNCOMPRESSED_FRAME_NUM}};

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
        assert_eq!(frame.unwrap().sum(), 794
    );
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

        // First 100 frames
        let frames = reader.get_frames_intensity(
            &(0u64..100u64).collect::<Vec<u64>>(),
            None
        );
        println!("{:?}", frames);
        assert!(frames.is_ok());

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
    fn test_get_frames_flim() {
        let reader = SiffReader::open(TEST_FILE_PATH).unwrap();
        let frame_nums = [15u64, 35u64];
        let frames = reader.get_frames_flim(&frame_nums, None);
        assert!(frames.is_ok());

        let frame_nums = [12u64, 37u64];
        let frames = reader.get_frames_flim(&frame_nums, None);
        assert!(frames.is_ok());
        let (lifetime, intensity) = frames.unwrap();
        let just_intensity = reader.get_frames_intensity(&frame_nums, None).unwrap();
        println!("{:?}", reader.get_frame_metadata(&frame_nums).unwrap());

        assert_eq!(just_intensity, intensity);

        //println!("Lifetime : {:?}", lifetime);
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

    #[test]
    fn get_frame_metadata(){
        let reader = SiffReader::open(TEST_FILE_PATH).unwrap();
        let metadata = reader.get_frame_metadata(&[15, 35]);
        assert!(metadata.is_ok());
        let metadata = metadata.unwrap();
        assert_eq!(metadata.len(), 2);
        assert_eq!(
            FrameMetadata::frame_number_from_metadata_str(
                &metadata[0].metadata_string
            ),
            15
        );

        assert!(
            FrameMetadata::frame_time_epoch_from_metadata_str(
                &metadata[1].metadata_string
            ) > 1e16 as u64
        );
        assert_eq!(
            FrameMetadata::frame_number_from_metadata_str(
                &metadata[1].metadata_string
            ),
            35
        );
    }

    #[test]
    fn test_sum_roi_methods() {
        let reader = SiffReader::open(TEST_FILE_PATH).unwrap();

        let frames = [UNCOMPRESSED_FRAME_NUM as u64, COMPRESSED_FRAME_NUM as u64];
        //let frames = [15, 35];

        // Test the wrong size ROI
        let wrong_roi = Array2::<bool>::from_elem((212, 329), true);

        let sum = reader.sum_roi_flat(&wrong_roi.view(), &frames, None);
        assert!(sum.is_err());

        // Test the correct size ROI        
        let mut roi = Array2::<bool>::from_elem(reader.image_dims().unwrap().to_tuple(), true);
        
        let whole_sum = reader.sum_roi_flat(&roi.view(), &frames, None).unwrap();

        let image_itself = reader.get_frames_intensity(&frames, None).unwrap();

        let image_itself_as_u64 = image_itself.mapv(|x| x as u64);
        assert_eq!(whole_sum, image_itself_as_u64.sum_axis(Axis(1)).sum_axis(Axis(1)));
        
        // Set the ROI to false in the middle
        roi.slice_mut(s![roi.shape()[0]/4..3*roi.shape()[0]/4, ..]).fill(false);
        let lesser_sum = reader.sum_roi_flat(&roi.view(), &frames, None).unwrap();
        assert!(lesser_sum.iter().all(|&x| x < whole_sum.iter().sum::<u64>()));

        // And assert it's what you actually get from multiplying it in!
        let roi_sum = image_itself_as_u64.axis_iter(Axis(0)).map(
            |frame| frame.indexed_iter().filter(
                |(idx, _)| roi[*idx]
            ).map(|(_, &x)| x).sum::<u64>()
        ).collect::<Vec<_>>();

        assert_eq!(roi_sum, lesser_sum.to_vec());

        // Now let's test whether registration gives consistent answers
        let mut reg = HashMap::<u64, (i32, i32)>::new();
        reg.insert(frames[0] as u64, (-15, 12));
        reg.insert(frames[1] as u64, (6,9));

        let whole_sum_reg = reader.sum_roi_flat(&roi.view(), &frames, Some(&reg)).unwrap();

        let shifted_image = reader.get_frames_intensity(&frames, Some(&reg)).unwrap();

        let shifted_roi_sum = shifted_image.mapv(|x| x as u64).axis_iter(Axis(0)).map(
            |frame| frame.iter().zip(roi.iter()).filter(
                |(_, &roi)| roi
            ).map(|(x, _)| *x).sum::<u64>()).collect::<Vec<_>>();

        assert_eq!(shifted_roi_sum, whole_sum_reg.to_vec());
    }

    #[test]
    fn test_3d_roi_mask(){
        /************
         * Now for 3d!
         */
        let reader = SiffReader::open(BIG_FILE_PATH).unwrap();
        // First 10000 frames
        //let frames = [UNCOMPRESSED_FRAME_NUM as u64, COMPRESSED_FRAME_NUM as u64];
        //let frames = [14 as u64, 40 as u64];
        let frames = (0..5000).map(|x| x as u64).collect::<Vec<_>>();
        let frame_dims = reader.image_dims().unwrap().to_tuple();
        let n_planes = 4;

        let mut three_d_roi = Array3::<bool>::from_elem(
            (n_planes, frame_dims.0, frame_dims.1),
            true
        );

        for k in 0..n_planes {
            three_d_roi.slice_mut(s![k, k*frame_dims.0/4..((k+2) % 4)*frame_dims.0/4, ..]).fill(false);
        }

        let three_d_sum = reader.sum_roi_volume(&three_d_roi.view(), &frames, None).unwrap();

        let image_itself = reader.get_frames_intensity(&frames, None).unwrap();

        let image_itself_as_u64 = image_itself.mapv(|x| x as u64);
        

        let piecewise = image_itself_as_u64.axis_iter(Axis(0)).zip(three_d_roi.axis_iter(Axis(0)).cycle()).map(
            |(frame, roi_plane)| frame.indexed_iter().filter(
                |(idx, _)| roi_plane[*idx]
            ).map(|(_, &x)| x).sum::<u64>()
        ).collect::<Vec<_>>();

        //assert_eq!(three_d_sum.to_vec(), piecewise)
    }

    #[test]
    fn time_methods(){
        let reader = SiffReader::open(TEST_FILE_PATH).unwrap();
        let times = reader.get_experiment_timestamps(&[15, 35]);
        assert!(times.is_ok());
        let times = times.unwrap();
        assert_eq!(times.len(), 2);
        assert_ne!(times[0], times[1]);
        println!("Experiment times : {:?}", times);


        let times = reader.get_epoch_timestamps_laser(&[15, 35]);
        assert!(times.is_ok());
        let times = times.unwrap();
        assert_eq!(times.len(), 2);
        assert_ne!(times[0], times[1]);
        println!("Epoch time (laser) : {:?}", times);

        let times = reader.get_epoch_timestamps_system(&[15, 16, 205]);
        assert!(times.is_ok());
        let times = times.unwrap();
        assert_eq!(times.len(), 3);
        // only updates once every few seconds.
        assert_eq!(times[0], times[1]);
        assert_ne!(times[0], times[2]);

        let both_times = reader.get_epoch_timestamps_both(&[15, 16, 205]);
        assert!(both_times.is_ok());
        let both_times = both_times.unwrap();
        assert_eq!(both_times.shape(), &[2, 3]);
        println!("Both times : {:?}", both_times);
        assert_ne!(both_times[(0, 0)], both_times[(0, 1)]);
        assert_ne!(both_times[(0, 0)], both_times[(0, 2)]);
        assert_eq!(both_times[(1, 0)], both_times[(1, 1)]);
        assert_ne!(both_times[(1, 0)], both_times[(1, 2)]);


    }
}