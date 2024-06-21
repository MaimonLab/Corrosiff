# Corrosiff

A `Rust`-based reader for `.siff` file data. An excuse to learn
and use `Rust`, which I have to admit has been pretty enjoyable.

TODOS:

- `get_epoch_timestamps_both` doesn't error if `system` timestamps
don't exist! It will just crash! Because I don't use the `?` correctly.

- `C`-compatible `FFI`.

# Installation
---------------


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