//! Methods in this submodule deal with extracting a pixelwise or ROI-wide
//! empirical lifetime from the data stored in a frame of a `.siff` file.
use binrw::io::{Read, Seek};
use registered::{_load_flim_intensity_empirical_compressed_registered, _load_flim_intensity_empirical_uncompressed_registered};
use std::io::{Error as IOError, ErrorKind as IOErrorKind};
use ndarray::prelude::*;
use crate::{
    tiff::{
        IFD,
        TiffTagID::{StripOffsets, StripByteCounts, Siff},
        Tag,
    },
    CorrosiffError,
    data::image::
        flim::empirical_lifetime::unregistered::{
            _load_flim_array_empirical_uncompressed,
            _load_flim_array_empirical_compressed,
            _load_flim_intensity_empirical_compressed,
            _load_flim_intensity_empirical_uncompressed,
        },
};

mod unregistered;
mod registered;


/// Loads an array with the pixelwise empirical lifetime
/// from the frame pointed to by the IFD. The reader
/// is returned to its original position. This method
/// is private because you almost never will want the
/// empirical lifetime without getting intensity information.
/// 
/// ## Arguments
/// 
/// * `reader` - The reader with access to the siff file
/// (implements `Read` + `Seek`)
/// 
/// * `ifd` - The IFD pointing to the frame to load the lifetime from
/// 
/// * `array` - The array to load the lifetime into (2d view for one frame)
/// 
/// ## Example
/// 
/// ```rust, ignore
/// use ndarray::prelude::*;
/// use std::fs::File;
/// 
/// let mut f = File::open("file.siff").unwrap();
/// let file_format = FileFormat::parse_filetype(&mut f).unwrap();
/// let mut array = Array2::<f64>::zeros((50, 256,256));
/// 
/// let ifds = file_format.get_ifd_vec(&mut f);
/// 
/// for (i, ifd) in ifds.iter().enumerate() {
///     load_flim_array_empirical(
///         &mut f,
///         ifd,
///         &mut array.slice_mut(s![i, ..])
///     ).unwrap();
/// }
/// ```
fn _load_flim_array_empirical<ReaderT, I>(
    reader : &mut ReaderT,
    ifd : &I,
    array : &mut ArrayViewMut2<f64>
    ) -> Result<(), CorrosiffError> where I : IFD, ReaderT : Read + Seek {
    let pos = reader.stream_position()?;

    reader.seek(std::io::SeekFrom::Start(ifd.get_tag(StripOffsets)
        .ok_or(IOError::new(IOErrorKind::InvalidData, "Strip offset not found"))?
        .value().into()
    ))?;

    match ifd.get_tag(Siff).unwrap().value().into() {
        0 => {
            _load_flim_array_empirical_uncompressed(
                reader,
                binrw::Endian::Little, 
                (
                    &mut array.view_mut(),
                    ifd.get_tag(StripByteCounts).unwrap().value().into(),
                    ifd.height().unwrap().into() as u32,
                    ifd.width().unwrap().into() as u32,
                ))?;
        },
        1 => {
            _load_flim_array_empirical_compressed(
                reader,
                binrw::Endian::Little, 
                (
                    &mut array.view_mut(),
                    ifd.get_tag(StripByteCounts).unwrap().value().into(),
                    ifd.height().unwrap().into() as u32,
                    ifd.width().unwrap().into() as u32,
                ))?;
        },
        _ => {
            Err(IOError::new(IOErrorKind::InvalidData, "Invalid Siff tag value"))?;
        }
    }

    let _ = reader.seek(std::io::SeekFrom::Start(pos))?;
    Ok(())
}

/// Loads intensity and empirical lifetime arrays from the frame
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
/// * `lifetime` - The array to load the lifetime into (2d view for one frame)
/// 
/// * `intensity` - The array to load the intensity into (2d view for one frame)
/// 
/// ## Example
/// 
/// ```rust, ignore
/// use ndarray::prelude::*;
/// use std::fs::File;
/// TODO: Write me!
/// ```
/// 
pub fn load_flim_empirical_and_intensity_arrays<I: IFD, ReaderT : Read + Seek>(
    reader : &mut ReaderT,
    ifd : &I,
    lifetime : &mut ArrayViewMut2<f64>,
    intensity : &mut ArrayViewMut2<u16>,
    ) -> Result<(), CorrosiffError> {
    let pos = reader.stream_position()?;
    reader.seek(std::io::SeekFrom::Start(ifd.get_tag(StripOffsets)
        .ok_or(IOError::new(IOErrorKind::InvalidData, "Strip offset not found"))?
        .value().into()
    ))?;

    match ifd.get_tag(Siff).unwrap().value().into() {
        0 => {
            _load_flim_intensity_empirical_uncompressed(
                reader,
                binrw::Endian::Little, 
                (
                    &mut lifetime.view_mut(),
                    &mut intensity.view_mut(),
                    ifd.get_tag(StripByteCounts).unwrap().value().into(),
                    ifd.height().unwrap().into() as u32,
                    ifd.width().unwrap().into() as u32,
                ))?;
        },
        1 => {
            _load_flim_intensity_empirical_compressed(
                reader,
                binrw::Endian::Little, 
                (
                    &mut lifetime.view_mut(),
                    &mut intensity.view_mut(),
                    ifd.get_tag(StripByteCounts).unwrap().value().into(),
                    ifd.height().unwrap().into() as u32,
                    ifd.width().unwrap().into() as u32,
                ))?;
        },
        _ => {
            Err(IOError::new(IOErrorKind::InvalidData, "Invalid Siff tag value"))?;
        }
    }

    let _ = reader.seek(std::io::SeekFrom::Start(pos))?;
    Ok(())
}

pub fn load_flim_empirical_and_intensity_arrays_registered
<I : IFD, ReaderT : Read + Seek>(
    reader : &mut ReaderT,
    ifd : &I,
    lifetime : &mut ArrayViewMut2<f64>,
    intensity : &mut ArrayViewMut2<u16>,
    registration : (i32, i32),
    ) -> Result<(), CorrosiffError> {
    let pos = reader.stream_position()?;
    reader.seek(std::io::SeekFrom::Start(ifd.get_tag(StripOffsets)
        .ok_or(IOError::new(IOErrorKind::InvalidData, "Strip offset not found"))?
        .value().into()
    ))?;

    match ifd.get_tag(Siff).unwrap().value().into() {
        0 => {
            _load_flim_intensity_empirical_uncompressed_registered(
                reader,
                binrw::Endian::Little, 
                (
                    &mut lifetime.view_mut(),
                    &mut intensity.view_mut(),
                    ifd.get_tag(StripByteCounts).unwrap().value().into(),
                    ifd.height().unwrap().into() as u32,
                    ifd.width().unwrap().into() as u32,
                    registration,
                ))?;
        },
        1 => {
            _load_flim_intensity_empirical_compressed_registered(
                reader,
                binrw::Endian::Little, 
                (
                    &mut lifetime.view_mut(),
                    &mut intensity.view_mut(),
                    ifd.get_tag(StripByteCounts).unwrap().value().into(),
                    ifd.height().unwrap().into() as u32,
                    ifd.width().unwrap().into() as u32,
                    registration,
                ))?;
        },
        _ => {
            Err(IOError::new(IOErrorKind::InvalidData, "Invalid Siff tag value"))?;
        }
    }

    let _ = reader.seek(std::io::SeekFrom::Start(pos))?;
    Ok(())
}

/// Internal structure for testing and validating
/// reading flim data.
#[allow(dead_code)]
struct FlimArrayEmpirical<D> {
    intensity : ndarray::Array<u16,D>,
    empirical_lifetime : ndarray::Array<f64, D>,
    confidence : Option<ndarray::Array<f64, D>>,
} 

#[allow(dead_code)]
impl<D> FlimArrayEmpirical<D> {
    
    /// Single frame
    pub fn from_ifd<I : IFD>(_ifd : &I) -> Result<FlimArrayEmpirical<Dim<[usize ; 2]>>, CorrosiffError> {
        Err(CorrosiffError::NotImplementedError)
    }

    /// Volume, with requested shape produced with a `reshape` method
    pub fn from_ifds<I : IFD>(_ifds : &[&I], _shape : Option<D>) -> Result<FlimArrayEmpirical<D>, CorrosiffError> {
        Err(CorrosiffError::NotImplementedError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{TEST_FILE_PATH, COMPRESSED_FRAME_NUM, UNCOMPRESSED_FRAME_NUM};
    use crate::data::image::intensity::siff::load_array as load_array_intensity;

    #[test]
    fn load_compressed_arrival_only() {
        let mut f = std::fs::File::open(TEST_FILE_PATH).unwrap();
        let file_format = crate::tiff::FileFormat::parse_filetype(&mut f).unwrap();

        let ifds = file_format.get_ifd_vec(&mut f);
        // let shape = (ifds[COMPRESSED_FRAME_NUM].height().unwrap().into() as usize,
        //     ifds[COMPRESSED_FRAME_NUM].width().unwrap().into() as usize);
        let shape = (128,128);
        let mut array = Array2::<f64>::zeros(shape);

        _load_flim_array_empirical(&mut f, &ifds[COMPRESSED_FRAME_NUM], &mut array.view_mut()).unwrap();
    }

    #[test]
    fn load_uncompressed_arrival_only() {
        let mut f = std::fs::File::open(TEST_FILE_PATH).unwrap();
        let file_format = crate::tiff::FileFormat::parse_filetype(&mut f).unwrap();

        let ifds = file_format.get_ifd_vec(&mut f);
        // let shape = (ifds[UNCOMPRESSED_FRAME_NUM].height().unwrap().into() as usize,
        //     ifds[UNCOMPRESSED_FRAME_NUM].width().unwrap().into() as usize);
        let shape = (128,128);
        let mut array = Array2::<f64>::zeros(shape);

        _load_flim_array_empirical(&mut f, &ifds[UNCOMPRESSED_FRAME_NUM], &mut array.view_mut()).unwrap();
    }

    #[test]
    fn load_intensity_and_flim_together_test(){
        let mut f = std::fs::File::open(TEST_FILE_PATH).unwrap();
        let file_format = crate::tiff::FileFormat::parse_filetype(&mut f).unwrap();

        let ifds = file_format.get_ifd_vec(&mut f);
        let shape = (128,128);
        let mut lifetime = Array2::<f64>::zeros(shape);
        let mut intensity = Array2::<u16>::zeros(shape);

        load_flim_empirical_and_intensity_arrays(&mut f, &ifds[UNCOMPRESSED_FRAME_NUM], &mut lifetime.view_mut(), &mut intensity.view_mut()).unwrap();

        // Now check that they're the same as the arrival-only tests

        let mut lifetime_arrival = Array2::<f64>::zeros(shape);
        let mut intensity_alone = Array2::<u16>::zeros(shape);

        _load_flim_array_empirical(&mut f, &ifds[UNCOMPRESSED_FRAME_NUM], &mut lifetime_arrival.view_mut()).unwrap();
        load_array_intensity(&mut f, &ifds[UNCOMPRESSED_FRAME_NUM], &mut intensity_alone.view_mut()).unwrap();
        
        lifetime.iter().zip(lifetime_arrival.iter()).for_each(|(&x,&y)| {
            if (!x.is_nan()) | (!y.is_nan()) {assert_eq!(x,y);}
        });

        intensity.iter().zip(intensity_alone.iter()).for_each(|(&x,&y)| {
            assert_eq!(x,y);
        });

        // Now again for the compressed frame

        let mut lifetime = Array2::<f64>::zeros(shape);
        let mut intensity = Array2::<u16>::zeros(shape);

        load_flim_empirical_and_intensity_arrays(&mut f, &ifds[COMPRESSED_FRAME_NUM], &mut lifetime.view_mut(), &mut intensity.view_mut()).unwrap();

        // Now check that they're the same as the arrival-only tests

        let mut lifetime_arrival = Array2::<f64>::zeros(shape);
        let mut intensity_alone = Array2::<u16>::zeros(shape);

        _load_flim_array_empirical(&mut f, &ifds[COMPRESSED_FRAME_NUM], &mut lifetime_arrival.view_mut()).unwrap();
        load_array_intensity(&mut f, &ifds[COMPRESSED_FRAME_NUM], &mut intensity_alone.view_mut()).unwrap();

        lifetime.iter().zip(lifetime_arrival.iter()).for_each(|(&x,&y)| {
            if (!x.is_nan()) | (!y.is_nan()) {assert_eq!(x,y);}
        });

        intensity.iter().zip(intensity_alone.iter()).for_each(|(&x,&y)| {
            assert_eq!(x,y);
        });

    }
}
