use std::env;
use corrosiff;

const USE_MESSAGE : &str = "\x1b[31mUsage: siff_to_tiff <filename>\
    [-m <mode>] [-o <output_path>]\x1b[0m";

macro_rules! send_use_msg {
    () => {
        panic!("{}", USE_MESSAGE)
    };
}


/// Converts a `.siff` file to a `.tiff` file
/// 
/// If `-m` not specified, uses ScanImage format.
/// If `-o` not specified, uses the same path as
/// the input file.
/// 
/// # Example
/// 
/// ```
/// siff_to_tiff my_siff.siff -m OME -o my_tiff.tiff
/// ```
fn main(){
    let args : Vec<String> = env::args().collect();
    if args.len() < 2 {send_use_msg!();}
    let filename = &args[1];
    let mut mode = None;
    let mut save_path = None;

    while let Some(arg) = args.iter().next() {
        match arg.as_str() {
            "-m" => {
                mode = Some(args.iter().next().unwrap_or_else(|| send_use_msg!()));
            },
            "-o" => {
                save_path = Some(args.iter().next().unwrap_or_else(|| send_use_msg!()));
            },
            _ => (),
        }
    }

    corrosiff::siff_to_tiff(
        filename,
        corrosiff::TiffMode::from_string_slice(mode.unwrap_or(&"ScanImage".to_string())).unwrap(),
        save_path
    ).expect("Failure in siff_to_tiff's implementation");    
}