//! Contains the Image File Directory (IFD) data structure
//! and the TiffTag and BigTag structures that are used
//! to read the tags in the IFD. To be honest I expect
//! this implementation to slow things down unnecessarily,
//! because I only use the BigTiff format, but this is partly
//! an exercise in learning Rust!
//! 
//! This _should_ be done with `traits`, the way
//! I'm starting to set it up. I will do it eventually...
//! 
//! NYPD KKK IFD they're all the same

use std::fmt::Debug;
use std::iter::Iterator;
use binrw::{
    io::{Read, Seek, SeekFrom},
    BinRead, //BinWrite,
    meta::ReadEndian
};

use crate::{
    tiff::tags::{TiffTag, TiffTagID, BigTag, Tag},
    data::image::Dimensions,
};

pub trait SeekRead : Seek + Read + Sized{}
impl<T : Seek + Read + Sized> SeekRead for T {}


/// Generic IFD trait for the `Tiff`, `BigTiff`, and `siff` formats
pub trait IFD : BinRead + ReadEndian + Default {
    type ByteSize : Into<u64> + Copy;
    type TagType : Tag;
    /// Both types of Tiff spec use a type that can
    /// be created from a u32, but not both types can
    /// be cast into one
    type PointerSize : Into<u64> + Copy + From<u32>;

    /// Creates a new `IFD` object from a reader
    /// pointing to the start of the IFD
    /// 
    /// ## Arguments
    /// 
    /// * `reader` - A reader pointing to the start of the IFD
    /// implementing `Seek` and `Read`
    fn new<T : Read + Seek>(reader : &mut T)->binrw::BinResult<Self>
        where for<'b> <Self as BinRead>::Args<'b>: Default{
        Self::read(reader)
    }

    /// Returns the number of tags stored by the IFD
    fn num_tags(&self) -> u16;
    
    /// Returns the location of the next IFD in the file
    fn next_ifd(&self) -> Option<Self::PointerSize>;
    
    /// Returns the width of the frame this IFD corresponds to
    fn width(&self)->Option<Self::PointerSize>;

    /// Returns the height of the frame this IFD corresponds to
    fn height(&self)->Option<Self::PointerSize>;

    /// If `height` and `width` are both valid, returns
    /// a `Dimensions` object containing the dimensions
    /// of the frame this IFD corresponds to.
    fn dimensions(&self)->Option<Dimensions> {
        match (self.width(), self.height()) {
            (Some(w), Some(h)) => {
                Some(Dimensions::new(w.into(), h.into()))
            },
            _ => None,
        }
    }

    /// Returns an object implementing the `TiffTag` trait corresponding
    /// whose `TiffTagID` matches that provided
    /// 
    /// ## Arguments
    /// 
    /// * `tag_id` - The `TiffTagID` of the tag to retrieve
    /// 
    /// ## Returns
    /// 
    /// * `Option<&Self::TagType>` - A reference to the tag if it exists
    /// in the IFD
    /// 
    /// ## Example
    /// 
    /// ```rust, ignore
    /// ```
    fn get_tag(&self, tag_id : TiffTagID)->Option<&Self::TagType>;

    /// Returns an object implementing the `TiffTag` trait using the
    /// corresponding string name of the tag
    /// 
    /// ## Arguments
    /// 
    /// * `str_slice` - The string name of the tag to retrieve
    /// 
    /// ## Returns
    /// 
    /// * `Option<&Self::TagType>` - A reference to the tag if it exists
    /// in the IFD
    /// 
    /// ## Example
    /// 
    /// ```rust, ignore
    /// println!("{}", tag)
    fn get_tag_from_str<'a>(&self, str_slice : &'a str)->Option<&Self::TagType>;

    /// Creates an iterator starting at the current IFD, and reads successive
    /// IFDs in the file
    fn to_iter<'a, T : Read + Seek>(&self, _reader : &'a mut T)->IFDIterator<'a, T, Self>{
        unimplemented!()
        // IFDIterator{
        //     reader,
        //     to_next : self.next_ifd(),
        // }
    }
}

/// Contains the IFD data, which is the
/// primary data structure for reading
/// tiff and siff files.
#[derive(BinRead, Default)]
#[br(little)]
pub struct TiffIFD {
    num_tags : u16,
    
    #[br(count = num_tags)]
    tags : Vec<TiffTag>,
    
    next_ifd : Option<u32>,
}

impl IFD for TiffIFD {
    type ByteSize = u16;
    type TagType = TiffTag;
    type PointerSize = u32;

    fn num_tags(&self) -> u16 {
        self.num_tags
    }
    fn next_ifd(&self) -> Option<u32> {
        self.next_ifd
    }
    fn width(&self)->Option<u32> {
        self.tags.iter()
            .find(|tag| tag.tag == TiffTagID::ImageWidth)
            .map(|tag| tag.value as u32)
    }

    fn height(&self)->Option<u32> {
        self.tags.iter()
            .find(|tag| tag.tag == TiffTagID::ImageLength)
            .map(|tag| tag.value as u32)
    }

    fn get_tag(&self, tag_id : TiffTagID)-> Option<&TiffTag>{
        self.tags.iter()
            .find(|tag| tag.tag == tag_id)
    }

    fn get_tag_from_str<'a>(&self, str_slice : &'a str)->Option<&Self::TagType> {
        self.tags.iter()
            .find(|tag| tag.tag.to_string() == str_slice)
    
    }
}


#[derive(BinRead, Default)]
#[br(little)]
pub struct BigTiffIFD {
    num_tags : u64,

    #[br(count = num_tags)]
    tags : Vec<BigTag>,

    pub next_ifd : Option<u64>,
}

impl IFD for BigTiffIFD {
    type ByteSize = u64;
    type TagType = BigTag;
    type PointerSize = u64;

    fn num_tags(&self) -> u16 {
        self.num_tags as u16
    }
    fn next_ifd(&self) -> Option<u64> {
        self.next_ifd
    }

    fn width(&self)->Option<u64> {
        self.tags.iter()
            .find(|tag| tag.tag == TiffTagID::ImageWidth)
            .map(|tag| tag.value)
    }

    fn height(&self)->Option<u64> {
        self.tags.iter()
            .find(|tag| tag.tag == TiffTagID::ImageLength)
            .map(|tag| tag.value)
    }

    fn get_tag(&self, tag_id : TiffTagID)->Option<&BigTag>{
        self.tags.iter()
            .find(|tag| tag.tag == tag_id)
    }

    fn get_tag_from_str<'b>(&self, str_slice : &'b str)->Option<&Self::TagType> {
        self.tags.iter()
            .find(|tag| tag.tag.to_string() == str_slice)
    }
}

/// The IFD values returned need guarantees to live longer than the transient
/// read operation.
pub struct IFDIterator<'reader, S, T> where S : SeekRead , T : IFD {
    pub reader : &'reader mut S,
    pub to_next : T::PointerSize,
}

impl <'a, S, IFDT> IFDIterator<'a, S, IFDT>
    where S : SeekRead, IFDT : IFD,
    for<'args> IFDT::Args<'args> : Default {

        /// TODO - Implement and test!! The generic needs to take in the
        /// IFD type?
        ///
        /// Creates a new `IFDIterator` object from an object that
        /// can read and seek and the location of the first IFD (so that
        /// it can parse it and find the subsequent IFDs).
        /// 
        /// ## Arguments
        /// 
        /// * `reader` - A reader that can read and seek
        /// * `first_ifd` - The location of the first IFD in the file
        /// 
        /// ## Returns
        /// 
        /// * `IFDIterator` - An iterator object that can be used to read
        /// 
        /// ## Example
        /// 
        /// ```rust, ignore
        /// ```
        fn new(reader : &'a mut S, first_ifd : IFDT::PointerSize) -> Self {
            IFDIterator{
                reader,
                to_next : first_ifd,
            }
        }
        
}

impl<'a, S, IFDT> Iterator for IFDIterator<'a, S, IFDT>
    where S : SeekRead, IFDT : IFD,
    for<'args> IFDT::Args<'args> : Default {
    type Item = IFDT;

    fn next(&mut self) -> Option::<Self::Item> {
        if self.to_next.into() == 0 {
            return None
        }
        self.reader.seek(SeekFrom::Start(self.to_next.into()))
            .ok()?;

        Some( IFDT::read(&mut self.reader).ok()? ).
            map(|ifd| {
                self.to_next = ifd.next_ifd().unwrap_or(IFDT::PointerSize::from(0 as u32));
                ifd
            }
        )
    }

    fn size_hint(&self) -> (usize, Option<usize>){
        (0, None)
    }
}   

impl Debug for TiffIFD {
    fn fmt(&self, f : &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "IFD: Num Tags: {}\nTags: {:?}\nNext IFD: {}",
            self.num_tags,
            self.tags,
            self.next_ifd.unwrap_or(0),
        )
    }
}

impl Debug for BigTiffIFD {
    fn fmt(&self, f : &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "IFD: Num Tags: {}\nTags: {:?}\nNext IFD: {}",
            self.num_tags,
            self.tags,
            self.next_ifd.unwrap_or(0),
        )
    }
}

#[cfg(test)]
mod tests {

    // use super::*;
    // use crate::tests::TEST_FILE_PATH;
}