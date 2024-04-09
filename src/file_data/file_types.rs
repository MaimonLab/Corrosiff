use std::io::BufRead;

#[derive(Debug)]
pub enum FileType {
    Valid(ValidType),
    Other,
}

#[derive(Debug)]
enum ValidType {
    Siff(Endian),
    FlimTiff(Endian),
    Tiff(Endian),
    Undetermined(Endian)
}

#[derive(Debug)]
enum Endian {
    Little,
    Big,
}

impl FileType{
    /// Checks a file against all the
    /// criteria to determine the enum
    /// type of file contained.
    /// 
    /// ## Arguments
    /// 
    /// * `buffer` - A BufRead pointing to
    /// the start of a file.
    /// 
    /// ## Example
    /// 
    /// ```
    /// use std::fs::File;
    /// use std::io::BufReader;
    /// 
    /// let file = File::open("file.siff")
    ///    .unwrap_or_else(
    ///       |_| panic!("Could not open file: {}", filename)
    ///   );
    /// 
    /// let mut buff = BufReader::new(file);
    /// 
    /// let filetype = FileType::discern_filetype(&mut buff);
    /// ```
    pub fn discern_filetype(buffer : &mut dyn BufRead) -> Self {
        let endian = FileType::get_endian(buffer)
            .expect(
            "Invalid endian specification"
        );
        FileType::Valid(ValidType::Undetermined(endian))
    }

    /// Checks the endian of a file by examining the
    /// first two bytes of the file. "II" is little endian,
    /// "MM" is big endian.
    /// 
    /// ## Arguments
    /// 
    /// * `buffer` - A BufRead pointing to
    /// the start of a file.
    /// 
    /// ## Example
    /// 
    /// ```
    /// let endian = FileType::get_endian(&file);
    /// ```
    /// 
    /// ## Returns
    /// 
    /// * `Some(Endian)` - If the endian is valid
    /// * `None` - If the endian is invalid
    /// 
    fn get_endian(buffer : &mut dyn BufRead) -> Option<Endian> {
        let mut endian: [u8; 2] = [0; 2];
        match buffer.read(&mut endian) {
            // if magic is "II" or "MM" then it's undetermined,
            // but Valid
            Ok(n) => {
                if n != 2 {
                    return None;
                }
                match &endian {
                    b"II" => Some(Endian::Little),
                    b"MM" => Some(Endian::Big),
                    _ => None
                }
            },
            Err(_) => None,
        }
    }

    // Returns a function that reads the file,
    // taking endian and filetype into account
    // fn read_func(&self) -> impl Fn(&mut dyn BufRead) -> () {
    //     match self {
    //         FileType::Valid(_) => |buffer| {
    //             println!("Valid file type");
    //         },
    //         FileType::Other => |buffer| {
    //             println!("Other file type");
    //         }
    //     }
    // }
}

impl ValidType{
}