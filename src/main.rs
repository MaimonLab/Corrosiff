use corrosiff;

fn main() {

    corrosiff::open_siff(
        "/Users/stephen/Desktop/Data/imaging/2024-04/2024-04-07/Dh31_LexA_LKir_LGFlamp1/Fly1/BarOnAtTen_1.siff"
    ).map(|siff| {
        println!("Filename: {}", siff.filename());
    }).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
    });
}