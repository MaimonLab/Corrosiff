use bytemuck::try_cast_slice;
use ndarray::prelude::*;

use std::io::{
    Error as IOError,
    ErrorKind as IOErrorKind,
};

use crate::data::image::{
    intensity::siff::unregistered::load_array_compressed_siff,
    dimensions::{
        macros::*,
        roll_inplace,
        roll
    },
};



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
pub fn load_array_raw_siff_registered<T : Into<u64>> (
    array : &mut ArrayViewMut2<u16>,
    strip_bytes : T,
    ydim : u32,
    xdim : u32,
    registration : (i32, i32),
    ) -> binrw::BinResult<()> {
    
    // let bytes = strip_bytes.into();
    // let mut data: Vec<u8> = vec![0; (8*((bytes/8) as usize)) as usize];
    let mut data: Vec<u8> = vec![0; strip_bytes.into() as usize];
    reader.read_exact(&mut data)?;

    try_cast_slice::<u8, u64>(&data)
    .map_err(|err| binrw::Error::Io(
        IOError::new(IOErrorKind::InvalidData, err))
    )?.iter().for_each(|siffphoton : &u64| {
        array[
            [
                photon_to_y!(siffphoton, registration.0, ydim),
                photon_to_x!(siffphoton, registration.1, xdim),
            ]
        ]+=1;
    });
    Ok(())
}

#[binrw::parser(reader)]
pub fn sum_mask_raw_siff_registered<T : Into<u64>>(
    frame_sum : &mut u64,
    mask : &ArrayView2<bool>,
    strip_bytes : T,
    ydim : u32,
    xdim : u32,
    registration : (i32, i32),
) -> binrw::BinResult<()> {
    // let bytes = strip_bytes.into();
    // let mut data: Vec<u8> = vec![0; (8*((bytes/8) as usize)) as usize];

    let mut data: Vec<u8> = vec![0; strip_bytes.into() as usize];
    reader.read_exact(&mut data)?;

    try_cast_slice::<u8, u64>(&data)
    .map_err(|err| binrw::Error::Io(
        IOError::new(IOErrorKind::InvalidData, err))
    )?.iter().for_each(|siffphoton : &u64| {
        *frame_sum += mask[
            [
                photon_to_y!(siffphoton, registration.0, ydim),
                photon_to_x!(siffphoton, registration.1, xdim),
            ]
        ] as u64;
    });
    Ok(())
}

/// Why doesn't ndarray have a roll method??
#[binrw::parser(reader, endian)]
pub fn load_array_compressed_siff_registered(
        array : &mut ArrayViewMut2<u16>,
        ydim : u32,
        xdim : u32,
        registration : (i32, i32),
    ) -> binrw::BinResult<()> {

    load_array_compressed_siff(reader, endian, (array, ydim, xdim))?;
    roll_inplace(array, registration);
    Ok(())
}

#[binrw::parser(reader)]
pub fn sum_mask_compressed_siff_registered(
    frame_sum : &mut u64,
    mask : &ArrayView2<bool>,
    ydim : u32,
    xdim : u32,
    registration : (i32, i32),
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

    // data.iter().zip(mask.iter()).for_each(|(&d, m)| {
    //     if *m {*frame_sum += d as u64}
    // });

    let mut data = Array2::<u16>::from_shape_vec((ydim as usize, xdim as usize), data.to_vec()).unwrap();

    roll_inplace(&mut data.view_mut(), registration);

    data.iter().zip(mask.iter()).for_each(|(&d, m)| {
        *frame_sum += (d as u64) * (*m as u64);
    });

    Ok(())
}
