//! TODO:
//! Make this actually use the `binrw` magics -- otherwise
//! why did I bother using all this `binrw` stuff to begin
//! with??

use binrw::io::{Read, Seek};
use bytemuck::try_cast_slice;
use ndarray::prelude::*;

use std::io::{
    Error as IOError,
    ErrorKind as IOErrorKind,
};

use crate::tiff::IFD;
use crate::tiff::{
    Tag,
    TiffTagID::{StripOffsets, StripByteCounts, Siff, },
};
use crate::data::image::dimensions::SIFF_YMASK as YMASK;
use crate::data::image::dimensions::SIFF_XMASK as XMASK;

/// Parses a `u64` from a photon in a raw `.siff` read
/// to the y coordinate of the photon. If a shift is
/// provided, it will add the shift to the y coordinate.
/// 
/// Can be called as:
/// 
/// ```rust, ignore
/// // One argument -- just a photon
/// let y = photon_to_y!(photon);
/// 
/// // Two arguments -- a photon and a y shift
/// // The resulting y coordinate is increased by
/// // the shift
/// let y = photon_to_y!(photon, shift);
/// ```
macro_rules! photon_to_y {
    ($photon : expr) => {
        (($photon & YMASK) >> 48) as usize
    };
    ($photon : expr, $shift : expr) => {
        (((($photon & YMASK) >> 48) as i32) + $shift) as usize
    };
}

/// Parses a `u64` from a photon in a raw `.siff` read
/// to the x coordinate of the photon. If a shift is
/// provided, it will add the shift to the x coordinate.
/// 
/// Can be called as:
/// 
/// ```rust, ignore
/// // One argument -- just a photon
/// let x = photon_to_x!(photon);
/// 
/// // Two arguments -- a photon and an x shift
/// // The resulting x coordinate is increased by
/// // the shift
/// let x = photon_to_x!(photon, shift);
/// ```
macro_rules! photon_to_x {
    ($photon : expr) => {
        (($photon & XMASK) >> 32) as usize
    };
    ($photon : expr, $shift : expr) => {
        (((($photon & XMASK) >> 32) as i32) + $shift) as usize
    };
}

/// Loads an allocated array with data read from a raw
/// `.siff` format frame (presumes the `reader` argument already
/// points to the frame) by ADDING data!
/// 
/// # Arguments
/// 
/// * `array` - The array to load the data into viewed as a 2d array
/// * `strip_bytes` - The number of bytes in the strip
/// * `ydim` - The height of the frame
/// * `xdim` - The width of the frame
/// 
/// # Example
/// 
/// ```rust, ignore
/// use ndarray::prelude::*;
/// use std::io::BufReader;
/// 
/// let mut array = Array2::<u16>::zeros((512, 512));
/// let mut reader = BufReader::new(std::fs::File::open("file.siff").unwrap());
/// reader.seek(std::io::SeekFrom::Start(34238)).unwrap();
/// load_array_raw_siff(&mut array, 512*512*2, 512, 512);
/// ```
/// 
/// # See also
/// 
/// `load_array_raw_siff_registered` - for loading an array
/// and shifting the data based on registration.
#[binrw::parser(reader)]
fn load_array_raw_siff<T : Into<u64>>(
        array : &mut ArrayViewMut2<u16>,
        strip_bytes : T,
        ydim : u32,
        xdim : u32,
    ) -> binrw::BinResult<()> {
        
    let mut data: Vec<u8> = vec![0; strip_bytes.into() as usize];
    reader.read_exact(&mut data)?;

    try_cast_slice::<u8, u64>(&data)
    .map_err(|err| binrw::Error::Io(
        IOError::new(IOErrorKind::InvalidData, err))
    )?.iter().for_each(|siffphoton : &u64| {
        array[
            [photon_to_y!(siffphoton) % (ydim as usize),
            photon_to_x!(siffphoton) % (xdim as usize),
            ]
        ]+=1;
    });
    Ok(())
}


/// Loads an allocated array with data read from a raw
/// `.siff` format frame (presumes the `reader` argument already
/// points to the frame) by ADDING data!
/// 
/// # Arguments
/// 
/// * `array` - The array to load the data into viewed as a 2d array
/// * `strip_bytes` - The number of bytes in the strip
/// * `ydim` - The height of the frame
/// * `xdim` - The width of the frame
/// * `registration` - A tuple of the pixelwise shifts, (y,x)
/// 
/// # Example
/// 
/// ```rust, ignore
/// use ndarray::prelude::*;
/// use std::io::BufReader;
/// 
/// let mut array = Array2::<u16>::zeros((512, 512));
/// let mut reader = BufReader::new(std::fs::File::open("file.siff").unwrap());
/// reader.seek(std::io::SeekFrom::Start(34238)).unwrap();
/// load_array_raw_siff_registered(&mut array, 512*512*2, 512, 512, (2, 2))
/// ```
/// 
/// # See also
/// 
/// `load_array_raw_siff` - for loading an array
/// without registration (plausibly faster?)
#[binrw::parser(reader)]
fn load_array_raw_siff_registered<T : Into<u64>> (
    array : &mut ArrayViewMut2<u16>,
    strip_bytes : T,
    ydim : u32,
    xdim : u32,
    registration : (i32, i32),
    ) -> binrw::BinResult<()> {
    let mut data: Vec<u8> = vec![0; strip_bytes.into() as usize];
    reader.read_exact(&mut data)?;

    try_cast_slice::<u8, u64>(&data)
    .map_err(|err| binrw::Error::Io(
        IOError::new(IOErrorKind::InvalidData, err))
    )?.iter().for_each(|siffphoton : &u64| {
        array[
            [photon_to_y!(siffphoton, registration.0) % (ydim as usize),
             photon_to_x!(siffphoton, registration.1) % (xdim as usize),
            ]
        ]+=1;
    });
    Ok(())
}

/// Parses a raw `.siff` format frame and returns
/// an `Intensity` struct containing the intensity data.
#[binrw::parser(reader, endian)]
fn raw_siff_parser<T : Into<u64>>(
    strip_bytes : T,
    ydim : u32,
    xdim : u32
    ) -> binrw::BinResult<Array2<u16>> {
    let mut frame = Array2::<u16>::zeros(
        (ydim as usize, xdim as usize)
    );
    load_array_raw_siff(reader, endian, (&mut frame.view_mut(), strip_bytes, ydim, xdim))?;
    Ok(frame)
}

/// Parses a compressed `.siff` format frame and returns
/// an `Intensity` struct containing the intensity data.
/// 
/// Expected to be at the data strip, so it will go backwards by the size of the
/// intensity data and read that.
#[binrw::parser(reader)]
fn load_array_compressed_siff(
        array : &mut ArrayViewMut2<u16>,
        ydim : u32,
        xdim : u32
    ) -> binrw::BinResult<()> {
    
    reader.seek(std::io::SeekFrom::Current(
        -(ydim as i64 * xdim as i64 * std::mem::size_of::<u16>() as i64)
    ))?;
    
    let mut data : Vec<u8> = vec![0; 
        ydim as usize * xdim as usize * std::mem::size_of::<u16>()
    ];
    reader.read_exact(&mut data)?;

    let data = try_cast_slice::<u8, u16>(&data).map_err(|err| binrw::Error::Io(
        IOError::new(IOErrorKind::InvalidData, err))
    )?;

    array.assign(
        &mut Array2::<u16>::from_shape_vec(
            (ydim as usize, xdim as usize),
            data.to_vec())
        .map_err(|err| binrw::Error::Io(
            IOError::new(IOErrorKind::InvalidData, err))
        )?
    );
    Ok(())
}

#[binrw::parser(reader)]
fn load_array_compressed_siff_registered(
        array : &mut ArrayViewMut2<u16>,
        ydim : u32,
        xdim : u32,
        registration : (i32, i32)
    ) -> binrw::BinResult<()> {
    
    reader.seek(std::io::SeekFrom::Current(-(ydim as i64 * xdim as i64 * std::mem::size_of::<u16>() as i64)))?;
    
    let mut data : Vec<u8> = vec![0; 
        ydim as usize * xdim as usize * std::mem::size_of::<u16>()
    ];
    reader.read_exact(&mut data)?;

    let data = try_cast_slice::<u8, u16>(&data).map_err(|err| binrw::Error::Io(
        IOError::new(IOErrorKind::InvalidData, err))
    )?;

    let unregistered =  Array2::<u16>::from_shape_vec(
        (ydim as usize, xdim as usize),
        data.to_vec()
    ).map_err(|err| binrw::Error::Io(
        IOError::new(IOErrorKind::InvalidData, err))
    )?;

    //Store the shifted version of unregistered in array    
    match registration {
        // No shift
        (0, 0) => {
            // seems silly with an unnecessary copy
            array.assign(&unregistered);
        },

        // x only
        (0, x_shift) => {
            array.slice_mut(s![.., x_shift..]).assign(
                &unregistered.slice(
                    s![.., ..-x_shift]));
            
            array.slice_mut(s![.., ..x_shift]).assign(
                &unregistered.slice(
                    s![.., -x_shift..]));
        },

        // y only
        (y_shift, 0) => {
            array.slice_mut(s![y_shift.., ..]).assign(
                &unregistered.slice(
                    s![..-y_shift, ..]));

            array.slice_mut(s![..y_shift, ..]).assign(
                &unregistered.slice(
                    s![-y_shift.., ..]));
        },
        (y_shift, x_shift) => {
            array.slice_mut(s![y_shift.., x_shift..]).assign(
                &unregistered.slice(
                    s![..-y_shift, ..-x_shift]));
            
            array.slice_mut(s![..y_shift, ..x_shift]).assign(
                &unregistered.slice(
                    s![-y_shift.., -x_shift..]));
        }
    }

    Ok(())
}

/// Parses a compressed `.siff` format frame and returns
/// an `Intensity` struct containing the intensity data.
/// 
/// Expected to be at the data strip, so it will go backwards by the size of the
/// intensity data and read that.
#[binrw::parser(reader)]
fn compressed_siff_parser(
        ydim : u32,
        xdim : u32
    ) -> binrw::BinResult<Array2<u16>> {

    reader.seek(std::io::SeekFrom::Current(
        -(ydim as i64 * xdim as i64 * std::mem::size_of::<u16>() as i64)
    ))?;
     
    let mut data : Vec<u8> = vec![0; 
        ydim as usize * xdim as usize * std::mem::size_of::<u16>()
    ];
    reader.read_exact(&mut data)?;

    let data = try_cast_slice::<u8, u16>(&data).map_err(|err| binrw::Error::Io(
        IOError::new(IOErrorKind::InvalidData, err))
    )?;

    Ok(Array2::<u16>::from_shape_vec((ydim as usize, xdim as usize), data.to_vec())
    .map_err(|err| binrw::Error::Io(
        IOError::new(IOErrorKind::InvalidData, err))
    )?)
}

/// Loads an allocated array with data read directly
/// from a `.siff` file. Will NOT change the `Seek`
/// location of the reader.
/// 
/// ## Arguments
/// 
/// * `reader` - Any reader of a `.siff` file
/// 
/// * `ifd` - The IFD of the frame to load into
/// 
/// * `array` - The array to load the data into viewed as a 2d array
/// 
/// ## Example
/// 
/// ```rust, ignore
/// use ndarray::prelude::*;
/// use std::fs::File;
/// 
/// let mut array = Array2::<u16>::zeros((512, 512));
/// let mut reader = File::open("file.siff").unwrap());
/// 
/// load_array(&mut reader, &ifd, &mut array.view_mut());
/// ```
/// 
/// ## See also
/// 
/// * `load_array_registered` - for loading an array
/// and shifting the data based on registration.
pub fn load_array<'a, T, S>(
        reader : &'a mut T,
        ifd : &'a S,
        array : &'a mut ArrayViewMut2<u16>
    ) -> Result<(), IOError> where S : IFD, T : Read + Seek
    {
    let pos = reader.stream_position()?;
    reader.seek(
        std::io::SeekFrom::Start(
            ifd.get_tag(StripOffsets)
            .ok_or(
                IOError::new(IOErrorKind::InvalidData, 
                "Strip offset not found"
                )
            )?.value().into()
        )
    )?;

    match ifd.get_tag(Siff).unwrap().value().into() {
        0 => {
            load_array_raw_siff(reader, binrw::Endian::Little, 
                (&mut array.view_mut(),
                ifd.get_tag(StripByteCounts).unwrap().value(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                )
            )
        },
        1 => {
            load_array_compressed_siff(reader, binrw::Endian::Little,
                (&mut array.view_mut(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                )
            )
        },
        _ => {
            Err(binrw::Error::Io(IOError::new(IOErrorKind::InvalidData,
                "Invalid Siff tag"
                )
            ))
        }
    }.map_err(|err| IOError::new(IOErrorKind::InvalidData, err))?;
    reader.seek(std::io::SeekFrom::Start(pos))?;
    Ok(())
}
/// Loads an allocated array with data read directly
/// from a `.siff` file. Will NOT change the `Seek`
/// location of the reader.
/// 
/// # Arguments
/// 
/// * `reader` - Any reader of a `.siff` file
/// 
/// * `ifd` - The IFD of the frame to load into
/// the array
/// 
/// * `array` - The array to load the data into viewed as a 2d array
/// whose pixels will be filled with the intensity data
/// 
/// * `registration` - A tuple of the pixelwise shifts
/// to register the frame. The first element is the
/// shift in the y direction, and the second element
/// is the shift in the x direction. The shifts are
/// in the direct of the shift itself, i.e. a positive
/// registration in the y direction will shift the frame down.
/// 
/// # Example
/// 
/// ```rust, ignore
/// use ndarray::prelude::*;
/// use std::fs::File;
/// 
/// let mut array = Array2::<u16>::zeros((512, 512));
/// let mut reader = File::open("file.siff").unwrap());
/// // TODO finish annotating
/// //let ifd = BigTiffIFD::new
/// // shift the frame down by 2 pixels
/// let registration = (2, 0);
/// load_array_registered(&mut reader, &ifd, &mut array.view_mut(), registration);
/// ```
/// 
/// # See also
/// 
/// * `load_array` - for loading an array without registration
pub fn load_array_registered<'a, T, S>(
    reader : &'a mut T,
    ifd : &'a S,
    array : &'a mut ArrayViewMut2<u16>,
    registration : (i32, i32),    
) -> Result<(), IOError> where S : IFD, T : Read + Seek {
    
    let pos = reader.stream_position()?;
    
    reader.seek(
        std::io::SeekFrom::Start(
            ifd.get_tag(StripOffsets)
            .ok_or(
                IOError::new(IOErrorKind::InvalidData, 
                "Strip offset not found"
                )
            )?.value().into()
        )
    )?;

    match ifd.get_tag(Siff).unwrap().value().into() {
        0 => {
            load_array_raw_siff_registered(reader, binrw::Endian::Little, 
                (&mut array.view_mut(),
                ifd.get_tag(StripByteCounts).unwrap().value(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                registration,
                )
            )
        },
        1 => {
            load_array_compressed_siff_registered(reader, binrw::Endian::Little,
                (&mut array.view_mut(),
                ifd.height().unwrap().into() as u32,
                ifd.width().unwrap().into() as u32,
                registration,
                )
            )
        },
        _ => {
            Err(binrw::Error::Io(IOError::new(IOErrorKind::InvalidData,
                "Invalid Siff tag"
                )
            ))
        }
    }.map_err(|err| {
        reader.seek(std::io::SeekFrom::Start(pos));
        IOError::new(IOErrorKind::InvalidData, err)
    })?;

    reader.seek(std::io::SeekFrom::Start(pos))?;
    Ok(())
}

/// A local struct for reading directly.
/// Only used internally for testing.
pub struct SiffFrame{
    pub intensity : ndarray::Array2<u16>,
}

impl SiffFrame {
    /// Parses a frame from a `.siff` file being viewed by
    /// `reader` using the metadata in the `ifd` argument
    /// to return a `SiffFrame` struct containing the intensity.
    /// 
    /// Does not move the `Seek` position of the reader because it
    /// is restored to its original position after reading the frame,
    /// EXCEPT if it errors!.
    /// 
    /// ## Arguments
    /// 
    /// * `ifd` - The IFD of the frame to load
    /// 
    /// * `reader` - The reader of the `.siff` file
    /// 
    /// ## Returns
    /// 
    /// * `Result<SiffFrame, IOError>` - A `SiffFrame` struct containing the intensity data
    /// for the requested frame.
    /// 
    /// ## Errors
    /// 
    /// * `IOError` - If the frame cannot be read for any reason
    /// this will throw an `IOError` and will NOT return the reader
    /// to its original position!
    pub fn from_ifd<'a, 'b, I, ReaderT>(ifd : &'a I, reader : &'b mut ReaderT) 
    -> Result<Self, IOError> where I : IFD, ReaderT : Read + Seek {
        let cur_pos = reader.stream_position()?;

        reader.seek(
        std::io::SeekFrom::Start(
                ifd.get_tag(StripOffsets)
                .ok_or(
                IOError::new(IOErrorKind::InvalidData, "Strip offset not found")
                )?.value().into()
            )
        )?;

        let parsed = match ifd.get_tag(Siff).unwrap().value().into() {
            0 => {
                raw_siff_parser(reader, binrw::Endian::Little,
                (
                    ifd.get_tag(StripByteCounts).unwrap().value(),
                    ifd.height().unwrap().into() as u32,
                    ifd.width().unwrap().into() as u32,
                )
            )},
            1 => {
                compressed_siff_parser(reader, binrw::Endian::Little, 
                (
                    ifd.height().unwrap().into() as u32,
                    ifd.width().unwrap().into() as u32,
                )
            )},
            _ => {Err(
                binrw::error::Error::Io(IOError::new(
                    IOErrorKind::InvalidData, "Invalid Siff tag")
                ))
            }
        }
        .map_err(|err| {
            reader.seek(std::io::SeekFrom::Start(cur_pos));
            IOError::new(IOErrorKind::InvalidData, err)
        })?;

        reader.seek(std::io::SeekFrom::Start(cur_pos));

        Ok(SiffFrame {
            intensity : parsed
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{TEST_FILE_PATH, UNCOMPRESSED_FRAME_NUM, COMPRESSED_FRAME_NUM};
    use crate::tiff::BigTiffIFD;

    use crate::tiff::FileFormat;

    #[test]
    fn test_extract_intensity() {
        let mut f = std::fs::File::open(TEST_FILE_PATH).unwrap();

        let fformat = FileFormat::parse_filetype(&mut f).unwrap();
        let ifd_vec : Vec<BigTiffIFD> = fformat.get_ifd_iter(&mut f).collect();
        
        // Compressed
        assert_eq!(
            SiffFrame::from_ifd(&ifd_vec[COMPRESSED_FRAME_NUM], &mut f).unwrap().intensity.sum(),
            65426 // from SiffPy
        );

        // Uncompressed
        assert_eq!(
            SiffFrame::from_ifd(&ifd_vec[UNCOMPRESSED_FRAME_NUM], &mut f).unwrap().intensity.sum(),
            397 // from SiffPy
        );

        // Says the number of photons in this pointer is right for another frame
        assert_eq!(
            SiffFrame::from_ifd(&ifd_vec[UNCOMPRESSED_FRAME_NUM+1], &mut f).unwrap().intensity.sum(),
            ((&ifd_vec[UNCOMPRESSED_FRAME_NUM+1]).get_tag(StripByteCounts).unwrap().value() as u16) / 8
        );

    }

    /// Shifts but only in the forward direction, i.e.
    /// if the shift is positive, it takes `n..end` and if
    /// the shift is negative, it takes `end-n..end`
    macro_rules! safe_slice_front{
        ($shift_y : expr, $shift_x : expr) => {
            match ($shift_y, $shift_x) {
                (0, 0) => s![.., ..],
                (0, _) => s![.., $shift_x..],
                (_, 0) => s![$shift_y.., ..],
                (_, _) => s![$shift_y.., $shift_x..]
            }
        }
    }

    /// Shifts but only in the backward direction, i.e.
    /// if the shift is positive, it takes `0..end-n` and if
    /// the shift is negative, it takes `n..end`
    macro_rules! safe_slice_back{
        ($shift_y : expr, $shift_x : expr) => {
            match ($shift_y, $shift_x) {
                (0, 0) => s![.., ..],
                (0, _) => s![.., ..-$shift_x],
                (_, 0) => s![..-$shift_y, ..],
                (_, _) => s![..-$shift_y, ..-$shift_x]
            }
        }
    }

    /// A macro to apply a load function and compare it to the registered version.
    /// Does not test the wrap-around behavior -- just up to when the shift wraps around.
    /// ```rust, ignore
    /// test_shift! (
    ///     $shift_y : expr,
    ///     $shift_x : expr,
    ///     $unregistered : expr,
    ///     $registered : expr,
    ///     $func : expr
    /// ) => { ... }
    /// ```
    /// 
    /// # Arguments
    /// 
    /// * `$shift_y` - The shift in the y direction
    /// * `$shift_x` - The shift in the x direction
    /// * `$unregistered` - The unregistered frame
    /// * `$registered` - The registered frame
    /// * `$func` - The function to call to load the registered frame
    /// 
    /// To use:
    /// 
    /// ```rust, ignore
    /// test_shift!(6, 0, frame.intensity, registered, call_load!(load_array_registered, registered));
    /// ```
    macro_rules! test_shift {
        (
            $shift_y : expr,
            $shift_x : expr,
            $unregistered : expr,
            $registered : expr,
            $func : expr
        ) => {
            let registration : (i32, i32) = ($shift_y, $shift_x);
            $func(registration).unwrap();
            assert_eq!(
                $unregistered.slice(safe_slice_back!($shift_y, $shift_x)),
                $registered.slice(safe_slice_front!($shift_y, $shift_x))
            );
        }
    }

    #[test]
    fn test_register() {
        let mut f = std::fs::File::open(TEST_FILE_PATH).unwrap();
        
        let fformat = FileFormat::parse_filetype(&mut f).unwrap();

        let ifd_vec : Vec<BigTiffIFD> = fformat.get_ifd_iter(&mut f).collect();

        // Shift down
        let frame = SiffFrame::from_ifd(&ifd_vec[UNCOMPRESSED_FRAME_NUM], &mut f).unwrap();
        let mut registered = Array2::<u16>::zeros((128, 128));

        macro_rules! call_load {
            ($func : expr, $register : expr, $ifd : expr) => {
                |x| $func(&mut f, $ifd, &mut $register.view_mut(), x)
            }
        }

        test_shift!(6, 0, frame.intensity, registered, call_load!(load_array_registered, registered, &ifd_vec[14]));
        // Shift up
        registered.fill(0);
        test_shift!(-6, 0, frame.intensity, registered, call_load!(load_array_registered, registered, &ifd_vec[14]));
        // Shift right
        registered.fill(0);
        test_shift!(0, 6, frame.intensity, registered, call_load!(load_array_registered, registered, &ifd_vec[14]));

        registered.fill(0);
        test_shift!(6, -6, frame.intensity, registered, call_load!(load_array_registered, registered, &ifd_vec[14]));

        println!("{:?}", ifd_vec[COMPRESSED_FRAME_NUM]);

        let frame = SiffFrame::from_ifd(&ifd_vec[40], &mut f).unwrap();
        let mut registered = Array2::<u16>::zeros((128, 128));

        test_shift!(0, 0, frame.intensity, registered, call_load!(load_array_registered, registered, &ifd_vec[40]));
        
        // let registration: (i32, i32) = (0, 0);
        // registered.fill(0);
        // load_array_registered(&mut f, &ifd_vec[40], &mut registered.view_mut(), registration).unwrap();
        assert_eq!(frame.intensity, registered);

        registered.fill(0);
        test_shift!(0, 0, frame.intensity, registered, call_load!(load_array_registered, registered, &ifd_vec[40]));

        registered.fill(0);
        test_shift!(0, -6, frame.intensity, registered, call_load!(load_array_registered, registered, &ifd_vec[40]));

        registered.fill(0);
        test_shift!(6, 0, frame.intensity, registered, call_load!(load_array_registered, registered, &ifd_vec[40]));

        registered.fill(0);
        test_shift!(6, -4, frame.intensity, registered, call_load!(load_array_registered, registered, &ifd_vec[40]));
    }

    #[test]

    /// Tests the photon conversion macros with fake photons
    fn test_photon_parse() {
        let y : u16 = (((1 as u64) << 16)-1) as u16;
        let x : u16 = 1;
        let arrival : u32 = 1;
        let photon : u64 = (y as u64) << 48 | (x as u64) << 32 | arrival as u64;
        assert_eq!(photon_to_y!(photon as u64), y as usize);
        assert_eq!(photon_to_x!(photon as u64), x as usize);
    }

}