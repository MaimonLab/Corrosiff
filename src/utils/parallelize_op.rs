/// `parallelize_op!` macro for repeating an operation across
/// chunks, usually of an array or iterator.
///
/// - `parallelize_op!(chunk_size, iterator, op)` 
/// 
///     for repeating an operation on each element of an iterator,
///     but splitting it into chunks and parallelizing the operation.
///
///     <br>
/// 
/// - `parallelize_op!(array, chunk_size, frames, filename, op)`
///     
///     For loading an array along its slow axis in parallel.
///     Divides the array into chunks and parallelizes the operation
///     `op` on each chunk. The operation `op` should take a slice of
///     frames, a mutable reference to a chunk of the array along its 0th axis,
///     and able to accept a `std::fs::File`, with the signature 
///     `op(frames : &[u64], chunk : &mut ArrayBase, reader : &mut Reader)`. Opens
///     local copies of the file for reading.
/// 
///     <br>
/// 
/// - `parallelize_op!((array1, array2, ...), chunk_size, frames, filename, op)`
/// 
///    For loading multiple arrays in parallel for functions that require
///    more than one array to be loaded. The arrays should be passed as a tuple
///    and the operation should accept the same number of arrays as the tuple.
///    (i.e. `op(
///         frames : &[u64],
///         array1 : &mut ArrayBase,
///         array2 : &mut ArrayBase,
///         ...,
///         reader : &mut Reader
///    )`).
///    
///    The behavior of this macro should be conceived of as zipping together
///    the chunked arrays and allowed `op` to be called on each chunk of the
///    arrays in parallel (`op(frames, array1_chunk, array2_chunk, ..., reader)`).
/// 
///    _Warning! Behavior is not well-defined if the arrays do not have
///    the same shape!!_
/// 
///    <br>
/// 
/// - `parallelize_op!(array, chunk_size, frames, filename, reader_call, op)`
///     
///     Allows alternative implementations for creating the reader instead of
///     producing `&mut File` objects (e.g. `&mut BufReader<&File>`) using a 
///     closure expecting the `filename` arg.
macro_rules! parallelize_op {

    (   $chunk_size : literal,
        $iterator : expr,
        $ op : expr
    ) => {
        let n_threads = $iterator.len()/$chunk_size + 1;
        let remainder = $iterator.len() % n_threads;

        let mut offsets = vec![];
        let mut start = 0;
        for i in 0..n_threads {
            let end = start + $chunk_size + if i < remainder { 1 } else { 0 };
            offsets.push((start, end));
            start = end;
        }

        $iterator.into_par_iter().enumerate().try_for_each(
            |(chunk_idx, chunk)| -> Result<(), CorrosiffError> {
                let start = chunk_idx * $chunk_size;
                let end = ((chunk_idx + 1) * $chunk_size).min($iterator.len());
                $op(&chunk[start..end])
            }
        )?;
    };
    
    (   $array : ident,
        $chunk_size : literal,
        $frames : ident,
        $filename : expr,
        $op : expr
    ) => {
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
            |(chunk_idx, mut chunk)| -> Result<(), CorrosiffError> {
            // Get the frame numbers and ifds for the frames in the chunk
            let start = chunk_idx * $chunk_size;
            let end = ((chunk_idx + 1) * $chunk_size).min($frames.len());

            let local_frames = &$frames[start..end];
            let mut local_f = File::open(&$filename).unwrap();

            $op(local_frames, &mut chunk, &mut local_f)
            }
        )?;
    };

    // Multiple arrays as a leading tuple
    (   ( $($array : ident),+ ),
        $chunk_size : literal,
        $frames : ident,
        $filename : expr,
        $op : expr
    ) => {
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

        // // Create an array of chunks to parallelize
        // let array_chunks : Vec<_> = $array.axis_chunks_iter_mut(Axis(0), $chunk_size).collect();

        // Create separate axis chunks for all of the arrays in the tuple,
        // then zip them back into a tuple of corresponding chunks to pass
        // to the into_par_iter method. This is a tuple of chunked arrays,
        let array_chunks = izip!(
            $(
                $array.axis_chunks_iter_mut(Axis(0), $chunk_size)
            ),+
        ).collect::<Vec<_>>();

        array_chunks.into_par_iter().enumerate().try_for_each(
            |(chunk_idx, mut chunk_tuple)| -> Result<(), CorrosiffError> {
            // Get the frame numbers and ifds for the frames in the chunk
            let start = chunk_idx * $chunk_size;
            let end = ((chunk_idx + 1) * $chunk_size).min($frames.len());

            let local_frames = &$frames[start..end];
            let mut local_f = File::open(&$filename).unwrap();

            // Unpack the chunk tuple into separate mutable references
            let ($(ref mut $array),+) = chunk_tuple;

            // Call the operation on each array separately
            $op(local_frames, $( $array ),+, &mut local_f)

            // // Call the operation on each array separately
            //$op(local_frames, chunk_tuple , &mut local_f)
            }
        )?;
    }; 

    (   $array : ident,
        $chunk_size : literal,
        $frames : ident,
        $filename : expr,
        Axis($axis:literal),
        $op : expr
    ) => {
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
        let array_chunks : Vec<_> = $array.axis_chunks_iter_mut(Axis($axis), $chunk_size).collect();

        array_chunks.into_par_iter().enumerate().try_for_each(
            |(chunk_idx, mut chunk)| -> Result<(), CorrosiffError> {
            // Get the frame numbers and ifds for the frames in the chunk
            let start = chunk_idx * $chunk_size;
            let end = ((chunk_idx + 1) * $chunk_size).min($frames.len());

            let local_frames = &$frames[start..end];
            let mut local_f = File::open(&$filename).unwrap();

            $op(local_frames, &mut chunk, &mut local_f)
            }
        )?;
    };

    (   $array : ident,
        $chunk_size : literal,
        $frames : ident,
        $filename : expr,
        $reader_call : expr,
        Axis($axis:literal),
        $op : expr
    ) => {
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
        let array_chunks : Vec<_> = $array.axis_chunks_iter_mut(Axis($axis), $chunk_size).collect();

        array_chunks.into_par_iter().enumerate().try_for_each(
            |(chunk_idx, mut chunk)| -> Result<(), CorrosiffError> {
            // Get the frame numbers and ifds for the frames in the chunk
            let start = chunk_idx * $chunk_size;
            let end = ((chunk_idx + 1) * $chunk_size).min($frames.len());

            let local_frames = &$frames[start..end];
            let mut local_f = $reader_call($filename);

            $op(local_frames, &mut chunk, &mut local_f)
            }
        )?;
    };

    (   $array : ident,
        $chunk_size : literal,
        $frames : ident,
        $filename : expr,
        $reader_call : expr,
        $op : expr
    ) => {
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
            |(chunk_idx, mut chunk)| -> Result<(), CorrosiffError> {
            // Get the frame numbers and ifds for the frames in the chunk
            let start = chunk_idx * $chunk_size;
            let end = ((chunk_idx + 1) * $chunk_size).min($frames.len());

            let local_frames = &$frames[start..end];
            let mut local_f = $reader_call($filename);

            $op(local_frames, &mut chunk, &mut local_f)
            }
        )?;
    };
}

/// Not sure if I'm going to use this, seems like
/// too small a bit of code to bother wrapping in a
/// macro, though it feels repetitive.
macro_rules! _registration_dependent_op {
    ($registration : ident, $some_op : expr, $none_op : expr) => {
        match $registration {
            Some(reg) => {
                $some_op
            },
            None => {
                $none_op
            }
        }
        Ok(())
    };
}

pub (crate) use parallelize_op;
//pub (crate) use _registration_dependent_op;
