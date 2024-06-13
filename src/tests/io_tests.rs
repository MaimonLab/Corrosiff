use corrosiff;

#[test]
fn siff_to_tiff() {
    corrosiff::siff_to_tiff(
    "/Users/stephen/Desktop/Data/imaging/2024-04/2024-04-07/Dh31_LexA_LKir_LGFlamp1/Fly1/BarOnAtTen_1.siff",
    corrosiff::TiffMode::ScanImage,
    None,
    ).unwrap();
}