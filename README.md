# Corrosiff

A `Rust`-based reader for `.siff` file data. An excuse to learn
and use `Rust`, which I have to admit has been pretty enjoyable.

TODOS:

- A real README.

- `get_epoch_timestamps_both` doesn't error if `system` timestamps
don't exist! It will just crash! Because I don't use the `?` correctly.

- `C`-compatible `FFI`.

- More sophisticated macros so that I don't have to manually go through
each file to make every combination of registered / unregistered /
mask / masks + flim. This seems like exactly the type of thing the
`Rust` macro system is perfect for (once I understand it better!)

- Consider making more useful and interesting `Rust` structs for
return values, rather than just passing back `ndarray` objects. I
could wrap these in, for example, the `SiffFrame` struct buried deep
in the `data` submodule and then would have access to some faster
operations implemented in `Rust` rather than relying on my `Python`
interface to do everything complicated (though access to `numpy`
really is nice...)

# Installation
---------------

This is not yet a publicly available crate, so for now you have to download
it and install it yourself!

First clone the repository:



# Sections
-----------

## File data

## Image Time

## Image

## Metadata

# Troubleshooting
------------------

# Testing
----------

## Benchmarking

The `corrosiff` library was implemented to
make up for my poor `C/C++` skills. Some of the
`siffreadermodule` calls of the `Python` extension
module were slow -- much slower than I'd like. So
I decided I'd learn `Rust` and see how that changes.

This section documents the difference in speed between
the two on the tests explored in the `benches` directory.