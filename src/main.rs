use corrosiff::open_siff;

fn main() {
    match open_siff(
            "/Users/stephen/Desktop/Data/imaging/2024-04/2024-04-07/Dh31_LexA_LKir_LGFlamp1/Fly1/BarOnAtTen_1.siff"
        ) {
            Ok(reader) => {
                println!(
                    "Filetype is {:?}",
                    reader.filetype
                );
            },
            Err(e) => {
                println!(
                    "Error: {}",
                    e
                );
            }
        };
}