//! TODO: Reduce the boilerplate by inlining some code!

/// ```rust, ignore
/// load_array_from_siff(
///     reader,
///     ifd,
///     (raw_func, (raw_args)),
///     (compressed_func, (compressed_args))
/// )
/// ```
/// 
/// or
/// ```rust, ignore
/// load_array_from_siff(
///    reader,
///    ifd,
///    (raw_func, (raw_args)),
///    (compressed_func, (compressed_args)),
///    (tiff_func, (tiff_args))
/// )
/// ```
/// 
/// Parses a compressed `.siff` format frame by calling
/// the correct parser based on the `Siff` tag in the IFD.
/// 
/// Moves the reader to the start of the data strip after parsing,
/// then returns it to its original position at the end
macro_rules! load_array_from_siff {
    (
        $reader : ident, 
        $ifd : ident,
        ( $raw_func : ident, ($($raw_args : expr),*) ),
        ( $compressed_func : ident, ($($compressed_args : expr),*) )
    ) => {{
        let pos = $reader.stream_position()?;
        $reader.seek(
            std::io::SeekFrom::Start(
                $ifd.get_tag(StripOffsets)
                .ok_or(
                    IOError::new(IOErrorKind::InvalidData, 
                    "Strip offset not found"
                    )
                )?.value().into()
            )
        )?;

        match $ifd.get_tag(Siff).unwrap().value().into() {
            0 => {
                $raw_func($reader, binrw::Endian::Little, ( $($raw_args),* ) )
            },
            1 => {
                $compressed_func($reader, binrw::Endian::Little, ( $($compressed_args),* ))
            },
            _ => {
                Ok(())
                // Err(
                //     CorrosiffError::NotImplementedError(
                //         "Only raw and compressed .siff formats are supported"
                //         " This file appears not to have a .siff tag -- is it a .tiff?"
                //     )
                // )
                // Err(binrw::Error::Io(IOError::new(IOErrorKind::InvalidData,
                //     "Invalid Siff tag"
                //     )
                // ))
            }
        }.map_err(|err| {
            let _ = $reader.seek(std::io::SeekFrom::Start(pos));
            IOError::new(IOErrorKind::InvalidData, err)
        })?;

        $reader.seek(std::io::SeekFrom::Start(pos))?;
        Ok(())
    }};

    (
        $reader : ident, 
        $ifd : ident,
        ( $raw_func : ident, ($($raw_args : expr),*) ),
        ( $compressed_func : ident, ($($compressed_args : expr),*) ),
        ( $tiff_func : ident, ($($tiff_args : expr),*) )
    ) => {{
        let pos = $reader.stream_position()?;
        $reader.seek(
            std::io::SeekFrom::Start(
                $ifd.get_tag(StripOffsets)
                .ok_or(
                    IOError::new(IOErrorKind::InvalidData, 
                    "Strip offset not found"
                    )
                )?.value().into()
            )
        )?;

        match $ifd.get_tag(Siff) {
            Some(tag) => { match tag.value().into() {
                0 => {
                    $raw_func($reader, binrw::Endian::Little, ( $($raw_args),* ) )
                },
                1 => {
                    $compressed_func($reader, binrw::Endian::Little, ( $($compressed_args),* ))
                },
                _ => {
                Ok(())
                // Err(
                //     CorrosiffError::NotImplementedError(
                //         "Only raw and compressed .siff formats are supported"
                //         " This file appears not to have a .siff tag -- is it a .tiff?"
                //     )
                // )
                // Err(binrw::Error::Io(IOError::new(IOErrorKind::InvalidData,
                //     "Invalid Siff tag"
                //     )
                // ))
                }
            }},
            None => {
                $tiff_func($reader, binrw::Endian::Little, ( $($tiff_args),* ))
            }
        }.map_err(|err| {
            let _ = $reader.seek(std::io::SeekFrom::Start(pos));
            IOError::new(IOErrorKind::InvalidData, err)
        })?;

        $reader.seek(std::io::SeekFrom::Start(pos))?;
        Ok(())
    }};
}

/// ```rust, ignore
/// photonwise_op(
///     reader,
///     strip_bytes,
///     op : |photon : &u64|
/// )
/// ```
/// Read the raw-format .siff frame and apply an operation
/// to every photon. Consumes `strip_bytes`
macro_rules! photonwise_op (
    ($reader : ident, $strip_bytes : ident, $op : expr) => {
        let mut data : Vec<u8> = vec![0; $strip_bytes.into() as usize];
        $reader.read_exact(&mut data)?;

        unsafe {
            let (_, data, _) = data.align_to::<u64>();
            data.iter().for_each( $op );
        }
    }
);

pub (crate) use load_array_from_siff;
pub (crate) use photonwise_op;