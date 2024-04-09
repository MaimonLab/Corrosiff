/// The primary `SiffReader` object, which
/// parses files and extracts interesting
/// information and/or data.
use std::fs::File;
use std::io::{BufReader, Result};
use crate::file_data::file_types::FileType as FileType;

/// A struct for reading a `.siff` file
/// or a ScanImage-Flim `.tiff` file, 
/// reads the data, and returns ???
pub struct SiffReader {
    _file : File,
    _filename : String,
    pub filetype : FileType,
}

impl SiffReader{
    
    /// Opens a file
    /// 
    /// # Arguments
    /// 
    /// * `filename` - A string slice that holds the name of the file to open
    /// 
    /// # Example
    /// 
    /// ```
    /// let reader = SiffReader::open("file.siff");
    /// ```
    pub fn open(filename : &str) -> Result<Self> {
        let file = File::open(&filename)?;
        let mut buff = BufReader::new(&file);
        
        Ok(SiffReader {
            _filename : String::from(filename),
            filetype : FileType::discern_filetype(&mut buff),
            _file : file,
            }
        )
    }
    /// Copy internal `filename` field
    pub fn filename(&self) -> &str {
        &self._filename
    }
}