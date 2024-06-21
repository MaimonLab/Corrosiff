//! # Image Time
//! 
//! Contains the data structures and functions for
//! parsing the timestamps of `.siff` files and returning
//! useful structures.

pub enum ClockBase {
    Experiment,
    EpochLaser,
    EpochSystem,
    Both
}