//! The `Metadata` struct parses the microscope acquisition
//! parameters and metadata from the various strings they are
//! stored in scattered throughout the file. Most of the data
//! is read from the `nvfd` -- non-varying-frame-data -- string,
//! but information about frame times comes from data just before
//! each individual IFD (for example).

/// The `Metadata` struct is a `Rust`
/// object that holds important or relevant
/// metadata in a human-interpretable format.
pub struct Metadata {}