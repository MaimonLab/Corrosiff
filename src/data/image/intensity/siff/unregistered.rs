use bytemuck::{try_cast_slice};
use ndarray::prelude::*;

use std::io::{
    Error as IOError,
    ErrorKind as IOErrorKind,
};

use crate::data::image::dimensions::macros::*;


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
pub fn load_array_raw_siff<T : Into<u64>>(
        array : &mut ArrayViewMut2<u16>,
        strip_bytes : T,
        ydim : u32,
        xdim : u32,
    ) -> binrw::BinResult<()> {
    
    // let bytes = strip_bytes.into();
    // let mut data: Vec<u8> = vec![0; (8*((bytes/8) as usize)) as usize];

    let mut data = vec![0; strip_bytes.into() as usize];
    reader.read_exact(&mut data)?;

    try_cast_slice::<u8, u64>(&data)
    .map_err(|err| binrw::Error::Io(
        IOError::new(IOErrorKind::InvalidData, err))
    )?.iter().for_each(|siffphoton : &u64| {
        array[
            [photon_to_y!(siffphoton, 0 , ydim),
            photon_to_x!(siffphoton, 0, xdim),
            ]
        ]+=1;
    });
    Ok(())
}

/// Computes the sum of the intensity data in a raw frame
/// masked by a bool array and stores it by changing the
/// `frame_sum` argument.
#[binrw::parser(reader)]
pub fn sum_mask_raw_siff<T : Into<u64>>(
    frame_sum : &mut u64,
    mask : &ArrayView2<bool>,
    strip_bytes : T,
    ydim : u32,
    xdim : u32,
) -> binrw::BinResult<()> {
    
    // let bytes = strip_bytes.into();
    // let mut data: Vec<u8> = vec![0; (8*((bytes/8) as usize)) as usize];

    let mut data = vec![0; strip_bytes.into() as usize];
    reader.read_exact(&mut data)?;

    try_cast_slice::<u8, u64>(&data)
    .map_err(|err| binrw::Error::Io(
        IOError::new(IOErrorKind::InvalidData, err))
    )?.iter().for_each(|siffphoton : &u64| {
        *frame_sum += mask[
            (photon_to_y!(siffphoton, 0 , ydim),
            photon_to_x!(siffphoton, 0, xdim),)
        ] as u64
    });

    Ok(())
}
 
/// Parses a compressed `.siff` format frame and returns
/// an `Intensity` struct containing the intensity data.
/// 
/// Expected to be at the data strip, so it will go backwards by the size of the
/// intensity data and read that.
#[binrw::parser(reader)]
pub fn load_array_compressed_siff(
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

    array.iter_mut().zip(data.iter()).for_each(|(a, &v)| *a = v);

    Ok(())
}

/// Computes the sum of the intensity data in a compressed frame
/// masked by a bool array and stores it by changing the
/// `frame_sum` argument.
#[binrw::parser(reader)]
pub fn sum_mask_compressed_siff(
    frame_sum : &mut u64,
    mask : &ArrayView2<bool>,
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

        // data.iter().zip(mask.iter()).for_each(|(&d, m)| {
        //     if *m {*frame_sum += d as u64}
        // });
    
        data.iter().zip(mask.iter()).for_each(|(&d, m)| {
            *frame_sum += (d as u64) * (*m as u64);
        });
    
        Ok(())
}