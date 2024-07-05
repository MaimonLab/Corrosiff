//! Benchmarking for metadata methods, such as methods for retrieving
//! time data, event stamps, etc.
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use criterion::BenchmarkId;
use corrosiff;

const LONG_SIFF_PATH: &str = "/Users/stephen/Desktop/Data/imaging/2024-04/2024-04-17/21Dhh_GCaFLITS/Fly1/Flashes_1.siff";
const APPENDED_TEXT_FILE : &str = "/Users/stephen/Desktop/Data/imaging/2024-05/2024-05-27/L2Split_GCaFLITS_KCL/Fly1/KClApplication_1.siff";

/// Open multiple files, read either a few frames quickly with and without registration
/// (to compare overhead latency) and then many frames with and without registration
/// (to compare the actual effect of adding registration)
fn criterion_benchmark_frame_metadata(c: &mut Criterion) {
    let siffreader = corrosiff::open_siff(LONG_SIFF_PATH).unwrap();
    let mut read_bench = c.benchmark_group("Get metadata benchmarks");
    let frame_vec = Vec::<u64>::from_iter(0..40);
    read_bench.bench_with_input(
        BenchmarkId::new("Get metadata from 40 frames",
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.get_frame_metadata(frames).unwrap()))
        },
    );

    read_bench.bench_with_input(
        BenchmarkId::new("Scan timestamps",
            0,
        ),
        &(),
        |bench, _| {
            bench.iter(|| black_box(corrosiff::scan_timestamps(&LONG_SIFF_PATH).unwrap()))
        },
    );

    read_bench.bench_with_input(
        BenchmarkId::new("Get 40 experiment timestamps",
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.get_experiment_timestamps(frames).unwrap()))
        },
    );

    read_bench.bench_with_input(
        BenchmarkId::new("Get 40 epoch laser stamps",
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.get_epoch_timestamps_laser(frames).unwrap()))
        },
    );

    read_bench.bench_with_input(
        BenchmarkId::new("Get 40 epoch system calls",
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.get_epoch_timestamps_system(frames).unwrap()))
        },
    );

    read_bench.bench_with_input(
        BenchmarkId::new("Get both epoch timestamps for 40 frames",
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.get_epoch_timestamps_both(frames).unwrap()))
        },
    );


    
    let frame_vec = Vec::<u64>::from_iter(0..49999);
    read_bench.sample_size(10);
    read_bench.bench_with_input(
        BenchmarkId::new("Get metadata from 50k-1 frames", 
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.get_frame_metadata(frames).unwrap()))
        },
    );

    read_bench.bench_with_input(
        BenchmarkId::new("Get 50k-1 experiment timestamps",
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.get_experiment_timestamps(frames).unwrap()))
        },
    );

    read_bench.bench_with_input(
        BenchmarkId::new("Get 50k-1 epoch laser stamps",
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.get_epoch_timestamps_laser(frames).unwrap()))
        },
    );

    read_bench.bench_with_input(
        BenchmarkId::new("Get 50k-1 epoch system calls",
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.get_epoch_timestamps_system(frames).unwrap()))
        },
    );

    read_bench.bench_with_input(
        BenchmarkId::new("Get both epoch timestamps for 50k-1 frames",
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.get_epoch_timestamps_both(frames).unwrap()))
        },
    );

    read_bench.bench_with_input(
        BenchmarkId::new("Search appended text where there is none",
            -1,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.get_appended_text(frames)))
        },
    );

    let new_reader = corrosiff::open_siff(APPENDED_TEXT_FILE).unwrap();

    let frames = Vec::<u64>::from_iter(0..new_reader.num_frames() as u64);
    read_bench.bench_with_input(
        BenchmarkId::new("Search appended text where there is some",
            -1
        ),
        &frames.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(new_reader.get_appended_text(frames)))
        },
    );
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = criterion_benchmark_frame_metadata,
);
criterion_main!(benches);