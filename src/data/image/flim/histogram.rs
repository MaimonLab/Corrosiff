//! TODO: Masked histogram methods!

use binrw::io::{Read, Seek};
use bytemuck::try_cast_slice;
use ndarray::prelude::*;

use std::io::{
    Error as IOError,
    ErrorKind as IOErrorKind,
};

use crate::{tiff::{
    Tag, TiffTagID::{Siff, StripByteCounts, StripOffsets }, IFD
}, CorrosiffError};
use crate::data::image::dimensions::macros::*;

/// Reads the data pointed to by the IFD and uses it to
/// increment the counts of the histogram. Presumes
/// the reader already points to the start of the main data.
fn _load_histogram_compressed<I, ReaderT>(
    ifd : &I,
    reader : &mut ReaderT,
    histogram : &mut ArrayViewMut1<u64>
    ) -> Result<(), IOError> 
    where I : IFD, ReaderT : Read + Seek {

    let strip_byte_counts = ifd.get_tag(StripByteCounts).unwrap().value();
    
    let mut data: Vec<u8> = vec![0; strip_byte_counts.into() as usize];
    reader.read_exact(&mut data)?;
    
    // confusing that the `if` statement is needed!!
    // come back to this! Maybe there's a mistake in how
    // the data is being saved??? Or maybe sometimes the laser
    // sync is missed, like one in every several thousand pulses?
    try_cast_slice::<u8, u16>(&data).map_err(
        |err| IOError::new(IOErrorKind::InvalidData, err)
    )?.iter().for_each(|&x| if (x < histogram.len() as u16) {histogram[x as usize] += 1});

    Ok(())
}

/// Presumes the reader is already at the start of the data
fn _load_histogram_uncompressed<I, ReaderT>(
    ifd : &I,
    reader : &mut ReaderT,
    histogram : &mut ArrayViewMut1<u64>
    ) -> Result<(), IOError> 
    where ReaderT : Read + Seek, I : IFD{

    let strip_byte_counts = ifd.get_tag(StripByteCounts).unwrap().value();
    let mut data : Vec<u8> = vec![0; strip_byte_counts.into() as usize];
    reader.read_exact(&mut data)?;

    try_cast_slice::<u8, u64>(&data).map_err(
        |err| IOError::new(IOErrorKind::InvalidData, err)
    )?.iter().for_each(|&x| {
        let tau = photon_to_tau_USIZE!(x);
        if tau < histogram.len() {histogram[tau] += 1}
    });
    Ok(())
}

/// Takes an existing array viewed in 1 dimension (presumed to be the tau dimension)
/// and loads the data from the frame pointed to by the current IFD.
/// 
/// Will NOT change the position of the reader.
/// 
/// ## Arguments
/// 
/// * `ifd` - The IFD pointing to the frame to load the histogram from
/// 
/// * `reader` - The reader with access to the data
/// 
/// * `histogram` - The array to load the histogram into (1d)
/// 
/// ## Example
/// 
/// ```rust, ignore
/// use ndarray::prelude::*;
/// use std::fs::File;
/// 
/// let mut f = File::open("file.siff").unwrap();
/// let file_format = FileFormat::parse_filetype(&mut f).unwrap();
/// let mut array = Array2::<u64>::zeros((50, file_format.num_flim_tau_bins().unwrap()));
/// let ifds = file_format.get_ifd_vec(&mut f);
/// 
/// for (i, ifd) in ifds.iter().enumerate() {
///    load_histogram(ifd, &mut f, &mut array.slice_mut(s![i, ..])).unwrap();
/// }
/// ```
pub fn load_histogram<I, ReaderT>(
    ifd: &I, reader: &mut ReaderT, histogram : &mut ArrayViewMut1<u64>
    )-> Result<(), IOError> where I : IFD, ReaderT : Read + Seek {
    let curr_pos = reader.stream_position()?;
    reader.seek(
        std::io::SeekFrom::Start(
            ifd.get_tag(StripOffsets)
            .ok_or(IOError::new(IOErrorKind::InvalidData,
            "Strip offset not found")
            )?.value().into()
        )  
    )?;
    match ifd.get_tag(Siff).unwrap().value().into() {
        0 => {
            _load_histogram_uncompressed(ifd, reader, histogram)?;
        },
        1 => {
            _load_histogram_compressed(ifd, reader, histogram)?;
        },
        _ => {
            Err(IOError::new(IOErrorKind::InvalidData,
                "Invalid Siff tag value"))?;
        }
    }
    let _ = reader.seek(std::io::SeekFrom::Start(curr_pos));
    Ok(())
}

/// Probably will contain more info at
/// some point...
#[allow(dead_code)]
struct FlimHistogram {
    data : Array1<u64>,
}

#[allow(dead_code)]
impl FlimHistogram {

    /// Create a new FlimHistogram from a given IFD
    /// 
    /// ## Arguments
    /// 
    /// * `ifd` - The IFD for the frame to create the histogram from
    ///
    /// * `reader` - The reader with access to the data
    ///
    /// ## Returns
    /// 
    /// A new FlimHistogram 
    fn from_ifd<'a, 'b, I, ReaderT>(ifd : &'a I, reader : &'b mut ReaderT, n_bins : u32)
    -> Result<Self, IOError> where I : IFD, ReaderT : Read + Seek {
        let curr_pos = reader.stream_position()?;

        reader.seek(
            std::io::SeekFrom::Start(
                ifd.get_tag(StripOffsets)
                .ok_or(IOError::new(IOErrorKind::InvalidData,
                "Strip offset not found")
                )?.value().into()
            )  
        )?;

        let mut hist = FlimHistogram {
            data : Array1::zeros(Dim(n_bins as usize)),
        };

        match ifd.get_tag(Siff).unwrap().value().into() {
            0 => {
                _load_histogram_uncompressed(ifd, reader, &mut hist.data.view_mut())?;
            },
            1 => {
                _load_histogram_compressed(ifd, reader, &mut hist.data.view_mut())?;
            },
            _ => {
                let _ = reader.seek(std::io::SeekFrom::Start(curr_pos));
                Err(IOError::new(IOErrorKind::InvalidData,
                    "Invalid Siff tag value"))?;
            }
        }
        
        let _ = reader.seek(std::io::SeekFrom::Start(curr_pos));
        Ok(hist)
    }
}

/// This is an image with an extra axis corresponding to
/// the arrival time of the photons in each pixel. The
/// fastest axis is the "tau" axis, corresponding to the
/// number of photons arriving in bin `tau` in that pixel
/// in the frame. For most FLIM data, there are ~1000 arrival
/// time bins, so these data RAPIDLY become gigantic.
#[allow(dead_code)]
struct ImageHistogram<D> {
    data : ndarray::Array<u64, D>
}

#[allow(dead_code)]
impl<D> ImageHistogram<D> {

    pub fn new_from_ifds<I : IFD>(ifds : &[&I]) -> Result<Self, CorrosiffError> {
        Err(CorrosiffError::NotImplementedError)
    }
}


#[cfg(test)]
mod tests{
    use super::*;
    use crate::tests::{
        TEST_FILE_PATH,
        UNCOMPRESSED_FRAME_NUM,
        COMPRESSED_FRAME_NUM
    };
    use crate::tiff::FileFormat;
    use crate::data::image::intensity::siff::SiffFrame;

    #[test]
    fn single_frame_histograms() {
        let mut f = std::fs::File::open(TEST_FILE_PATH).unwrap();

        let file_format = FileFormat::parse_filetype(&mut f).unwrap();
        let ifd_vec = file_format.get_ifd_vec(&mut f);

        println!("Loading compressed data");
        let hist = FlimHistogram::from_ifd(
            &ifd_vec[COMPRESSED_FRAME_NUM], 
            &mut f, 
            file_format.num_flim_tau_bins().unwrap()
        );

        // WEIRD!!
        let frame = SiffFrame::from_ifd(&ifd_vec[COMPRESSED_FRAME_NUM], &mut f).unwrap();
        println!("{:?} photons ", frame.intensity.sum());
        println!("{:?}", hist.unwrap().data.sum());

        println!("Loading uncompressed data");
        let hist = FlimHistogram::from_ifd(
            &ifd_vec[UNCOMPRESSED_FRAME_NUM], 
            &mut f, 
            file_format.num_flim_tau_bins().unwrap()
        ).unwrap();

        let frame = SiffFrame::from_ifd(&ifd_vec[UNCOMPRESSED_FRAME_NUM], &mut f).unwrap();
        // Should have the same number of photons
        assert_eq!(hist.data.sum(), frame.intensity.fold(0, |running_sum, &x| running_sum + (x as u64)));
        println!{"{:?}", hist.data};
    }

    #[test]
    fn image_histogram_tests(){
        let mut f = std::fs::File::open(TEST_FILE_PATH).unwrap();
        let file_format = FileFormat::parse_filetype(&mut f).unwrap();
        let ifd_vec = file_format.get_ifd_vec(&mut f);

        let mut hist = ImageHistogram {
            data : ArrayD::<u64>::zeros(IxDyn(&[file_format.num_flim_tau_bins().unwrap() as usize, 512, 512]))
        };

        let mut reader = std::io::BufReader::new(f);
        for (i, ifd) in ifd_vec.iter().enumerate() {
            let curr_pos = reader.stream_position().unwrap();
            reader.seek(
                std::io::SeekFrom::Start(
                    ifd.get_tag(StripOffsets)
                    .unwrap().value().into()
                )
            ).unwrap();
            // TODO finish test
            assert!(false);
            // match ifd.get_tag(Siff).unwrap().value().into() {
            //     0 => {
            //         _load_histogram_uncompressed(ifd, &mut reader, &mut hist.data.index_axis_mut(Axis(0), i)).unwrap();
            //     },
            //     1 => {
            //         _load_histogram_compressed(ifd, &mut reader, &mut hist.data.index_axis_mut(Axis(0), i)).unwrap();
            //     },
            //     _ => {
            //         panic!("Invalid Siff tag value");
            //     }
            // }
            reader.seek(std::io::SeekFrom::Start(curr_pos)).unwrap();
        }
    }
}

