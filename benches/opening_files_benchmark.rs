use criterion::{black_box, criterion_group, criterion_main, Criterion};
use corrosiff::{self, SiffReader, CorrosiffError};

const MOUNT_SIFF_PATH :&str = "/Users/stephen/maimondata01/Stephen/Imaging/2024-04/2024-04-23/R70B05_GFlamp2/Fly1/BarOnAtTen_1.siff";

fn open_short_siff_test()->Result<SiffReader, CorrosiffError>{
    corrosiff::open_siff(
        "/Users/stephen/Desktop/Data/imaging/2024-04/2024-04-07/Dh31_LexA_LKir_LGFlamp1/Fly1/BarOnAtTen_1.siff"
    )
}

fn open_long_siff_test()->Result<SiffReader, CorrosiffError>{
    corrosiff::open_siff(
        "/Users/stephen/Desktop/Data/imaging/2024-04/2024-04-17/21Dhh_GCaFLITS/Fly1/Flashes_1.siff"
    )
}

fn open_mount_siff()->Result<SiffReader, CorrosiffError>{
    corrosiff::open_siff(MOUNT_SIFF_PATH)
}

fn criterion_benchmark(c: &mut Criterion) {
    // c.bench_function("open a mounted siff file", 
    // |b| b.iter(|| black_box(open_mount_siff()))
    // );
    c.bench_function("open a short siff file",
    |b| b.iter(|| black_box(open_short_siff_test()))
    );
    c.bench_function("open a long siff file", |b| b.iter(|| black_box(open_long_siff_test())));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);