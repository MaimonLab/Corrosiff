use criterion::{black_box, criterion_group, criterion_main, Criterion};
use criterion::BenchmarkId;
use corrosiff;

use std::collections::HashMap;

const SHORT_SIFF_PATH: &str = "/Users/stephen/Desktop/Data/imaging/2024-04/2024-04-07/Dh31_LexA_LKir_LGFlamp1/Fly1/BarOnAtTen_1.siff";
const LONG_SIFF_PATH: &str = "/Users/stephen/Desktop/Data/imaging/2024-04/2024-04-17/21Dhh_GCaFLITS/Fly1/Flashes_1.siff";

/// Open multiple files, read either a few frames quickly with and without registration
/// (to compare overhead latency) and then many frames with and without registration
/// (to compare the actual effect of adding registration)
fn criterion_benchmark_read_flim(c: &mut Criterion) {
    let siffreader = corrosiff::open_siff(SHORT_SIFF_PATH).unwrap();
    let mut read_bench = c.benchmark_group("Flim reading benchmarks");
    let frame_vec = Vec::<u64>::from_iter(0..40);
    read_bench.bench_with_input(
        BenchmarkId::new("Read short siff, 40 frames unregistered", 
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.get_frames_flim(frames, None).unwrap()))
        },
    );

    let mut reg : HashMap<u64, (i32, i32)> = HashMap::new();
    
    frame_vec.iter().for_each(|&x| {
        reg.insert(x, ((x%100) as i32, ((x + 50) % 100) as i32));
    });

    read_bench.bench_with_input(
        BenchmarkId::new("Read short siff, 40 frames registered", 
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.get_frames_flim(frames, Some(&reg)).unwrap()))
        },
    );

    let siffreader = corrosiff::open_siff(LONG_SIFF_PATH).unwrap();
    let frame_vec = Vec::<u64>::from_iter(0..49999);
    read_bench.sample_size(10);
    read_bench.bench_with_input(
        BenchmarkId::new("Read long siff, 50k-1 frames unregistered", 
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.get_frames_flim(frames, None).unwrap()))
        },
    );

    let mut reg : HashMap<u64, (i32, i32)> = HashMap::new();
    
    frame_vec.iter().for_each(|&x| {
        reg.insert(x, ((x % 100) as i32, ((x + 50) % 100) as i32 ));
    });

    read_bench.bench_with_input(
        BenchmarkId::new("Read long siff, 50k-1 frames registered", 
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.get_frames_flim(frames, Some(&reg)).unwrap()))
        },
    );
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = criterion_benchmark_read_flim,
);
criterion_main!(benches);