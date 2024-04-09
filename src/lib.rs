/// Lib

use std::{
    io::Result,
    path::PathBuf,
};

mod file_data;
mod siffreader;

use file_data::TiffMode;

/// `open_siff(filename)` opens a `.siff` file
/// or a ScanImage-Flim `.tiff` file, reads the data,
/// and returns a `SiffReader` object.
/// 
/// ## Arguments
/// 
/// * `filename` - A string slice that holds the name of the file to open
/// 
/// ## Example
/// 
/// ```
/// let reader = open_siff("file.siff");
/// ```
pub fn open_siff(filename : &str) -> Result<siffreader::SiffReader> {
    siffreader::SiffReader::open(filename)
}

/// `siff_to_tiff(filename, mode)` converts a `.siff` file
/// to a `.tiff` file, using the specified mode. If the mode
/// is `TiffMode::ScanImage`, the main metadata remains
/// unconverted, and it uses the standard ScanImage format.
/// If the mode is `TiffMode::OME`, the metadata is converted
/// to the OME-TIFF format.
/// 
/// ## Arguments
/// 
/// * `filename` - A string slice that holds the name of the file to open
/// * `mode` - A `TiffMode` enum that specifies the conversion mode
/// * `save_path` - An optional string slice that holds the path
/// to save the converted file. If not specified, the file is saved
/// in the same directory as the original file, with the same name
/// but the extension `.tiff`.
/// 
/// ## Example
/// 
/// ```
/// // Produces "file.tiff" in OME-TIFF format
/// siff_to_tiff("file.siff", TiffMode::OME);
/// // Produces "file2.tiff" in ScanImage format
/// siff_to_tiff("file.siff", TiffMode::ScanImage, Some("file2.tiff"));
/// ```
pub fn siff_to_tiff(
    filename :& str,
    mode : TiffMode,
    save_path : Option<&str>,
    ) -> (){
    match mode {
        TiffMode::ScanImage => {
            println!("ScanImage mode");
        },
        TiffMode::OME => {
            println!("OME mode");
        }
    }

    let file_path: PathBuf = PathBuf::from(filename);

    let save_path: PathBuf = match save_path {
        Some(name) => PathBuf::from(name),
        None => file_path.with_extension("tiff"),
    };

}

mod front_of_house {
    pub mod hosting {
        pub fn add_to_waitlist() {
            println!("Just dummy code, you dummy");
        }
    }
}

pub fn eat_at_restaurant() {
    // Absolute path
    crate::front_of_house::hosting::add_to_waitlist();

    // Relative path
    front_of_house::hosting::add_to_waitlist();
}