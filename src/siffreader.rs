use std::file::File;

pub enum FileType {
    Siff,
    FlimTiff,
    Tiff,
}

pub struct SiffReader {
    file: File,
    filetype : FileType,
}

/// A struct for reading a `.siff` file
/// or a ScanImage-Flim `.tiff` file, 
/// reads the data, and returns ???
impl SiffReader {
    
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
    fn open(filename : &str) -> Self {
        let file = File::open(filename).unwrap();
        SiffReader {
            file: file,
        }

    }

    /// # Example
    /// 
    /// ```
    /// let reader = SiffReader::open("file.siff");
    /// reader.close();
    /// ```
    fn close(&self) {
        // close the file
    }
}

fn main() {
    let reader = SiffReader::open("file.siff");
}