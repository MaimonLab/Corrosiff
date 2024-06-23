use criterion::{black_box, criterion_group, criterion_main, Criterion};
use criterion::BenchmarkId;
use corrosiff;
use ndarray::prelude::*;
use rand;

use std::collections::HashMap;

const SHORT_SIFF_PATH: &str = "/Users/stephen/Desktop/Data/imaging/2024-04/2024-04-07/Dh31_LexA_LKir_LGFlamp1/Fly1/BarOnAtTen_1.siff";
const LONG_SIFF_PATH: &str = "/Users/stephen/Desktop/Data/imaging/2024-04/2024-04-17/21Dhh_GCaFLITS/Fly1/Flashes_1.siff";

/// Open multiple files, read either a few frames quickly with and without registration
/// (to compare overhead latency) and then many frames with and without registration
/// (to compare the actual effect of adding registration)
fn criterion_benchmark_read_one_mask(c: &mut Criterion) {

    //////////////////////// FLAT MASKS/////////////////
    let siffreader = corrosiff::open_siff(SHORT_SIFF_PATH).unwrap();
    let mut read_bench = c.benchmark_group("FLIM Mask sum benchmarks");
    let frame_vec = Vec::<u64>::from_iter(0..40);

    let frame_dims = siffreader.image_dims().unwrap().to_tuple();
    let mut mask = Array2::<bool>::from_elem(frame_dims, true);

    mask.slice_mut(s![frame_dims.0/4..3*frame_dims.0/4, ..]).fill(false);

    read_bench.bench_with_input(
        BenchmarkId::new("Flat mask short siff, 40 frames unregistered", 
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.sum_roi_flim_flat(&mask.view(), frames, None).unwrap()))
        },
    );

    let mut reg : HashMap<u64, (i32, i32)> = HashMap::new();
    
    frame_vec.iter().for_each(|&x| {
        reg.insert(x, ((x%100) as i32, ((x + 50) % 100) as i32));
    });

    read_bench.bench_with_input(
        BenchmarkId::new("Flat mask short siff, 40 frames registered", 
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.sum_roi_flim_flat(&mask.view(), frames, Some(&reg)).unwrap()))
        },
    );

    let siffreader = corrosiff::open_siff(LONG_SIFF_PATH).unwrap();
    let frame_vec = Vec::<u64>::from_iter(0..49999);

    let frame_dims = siffreader.image_dims().unwrap().to_tuple();
    let mut mask = Array2::<bool>::from_elem(frame_dims, true);

    mask.iter_mut().for_each(|x| *x = rand::random::<bool>());

    read_bench.sample_size(10);
    read_bench.bench_with_input(
        BenchmarkId::new("Flat mask long siff, 50k-1 frames unregistered", 
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.sum_roi_flim_flat(&mask.view(), frames, None).unwrap()))
        },
    );

    let mut reg : HashMap<u64, (i32, i32)> = HashMap::new();
    
    frame_vec.iter().for_each(|&x| {
        reg.insert(x, ((x % 100) as i32, ((x + 50) % 100) as i32 ));
    });

    read_bench.bench_with_input(
        BenchmarkId::new("Flat mask long siff, 50k-1 frames registered", 
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.sum_roi_flim_flat(&mask.view(), frames, Some(&reg)).unwrap()))
        },
    );

    /////////////////// 3d masks ///////////////////////
    
    let siffreader = corrosiff::open_siff(SHORT_SIFF_PATH).unwrap();
    let frame_vec = Vec::<u64>::from_iter(0..40);

    let frame_dims = siffreader.image_dims().unwrap().to_tuple();
    let mut mask = Array3::<bool>::from_elem(
        (10, frame_dims.0, frame_dims.1), true);

    mask.iter_mut().for_each(|x| *x = rand::random::<bool>());

    read_bench.bench_with_input(
        BenchmarkId::new("Volume mask short siff, 40 frames unregistered", 
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.sum_roi_flim_volume(&mask.view(), frames, None).unwrap()))
        },
    );

    let mut reg : HashMap<u64, (i32, i32)> = HashMap::new();
    
    frame_vec.iter().for_each(|&x| {
        reg.insert(x, ((x%100) as i32, ((x + 50) % 100) as i32));
    });

    read_bench.bench_with_input(
        BenchmarkId::new("Volume mask short siff, 40 frames registered", 
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.sum_roi_flim_volume(&mask.view(), frames, Some(&reg)).unwrap()))
        },
    );

    let siffreader = corrosiff::open_siff(LONG_SIFF_PATH).unwrap();
    let frame_vec = Vec::<u64>::from_iter(0..49999);

    let frame_dims = siffreader.image_dims().unwrap().to_tuple();
    let mut mask = Array3::<bool>::from_elem(
        (10, frame_dims.0, frame_dims.1), true);


    mask.iter_mut().for_each(|x| *x = rand::random::<bool>());

    read_bench.sample_size(10);
    read_bench.bench_with_input(
        BenchmarkId::new("Volume mask long siff, 50k-1 frames unregistered", 
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.sum_roi_flim_volume(&mask.view(), frames, None).unwrap()))
        },
    );

    let mut reg : HashMap<u64, (i32, i32)> = HashMap::new();
    
    frame_vec.iter().for_each(|&x| {
        reg.insert(x, ((x % 100) as i32, ((x + 50) % 100) as i32 ));
    });

    read_bench.bench_with_input(
        BenchmarkId::new("Volume mask long siff, 50k-1 frames registered", 
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.sum_roi_flim_volume(&mask.view(), frames, Some(&reg)).unwrap()))
        },
    );
}


///// Multimasks!!////
fn criterion_benchmark_read_multiple_masks(c: &mut Criterion) {

    //////////////////////// FLAT MASKS/////////////////
    let siffreader = corrosiff::open_siff(SHORT_SIFF_PATH).unwrap();
    let mut read_bench = c.benchmark_group("Multimask FLIM sum benchmarks");
    let frame_vec = Vec::<u64>::from_iter(0..40);

    let frame_dims = siffreader.image_dims().unwrap().to_tuple();
    let mut masks = Array3::<bool>::from_elem(
        (10, frame_dims.0, frame_dims.1), true
    );

    masks.iter_mut().for_each(|x| *x = rand::random::<bool>());

    read_bench.bench_with_input(
        BenchmarkId::new("Flat masks short siff, 40 frames unregistered", 
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.sum_rois_flim_flat(&masks.view(), frames, None).unwrap()))
        },
    );

    read_bench.bench_with_input(
        BenchmarkId::new("Flat masks individually short siff, 40 frames unregistered", 
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(||
                black_box(
                    masks.axis_iter(Axis(0)).for_each(|mask| {
                        siffreader.sum_roi_flat(&mask, frames, None).unwrap();
                    })
                )
            )
        },
    );

    let mut reg : HashMap<u64, (i32, i32)> = HashMap::new();
    
    frame_vec.iter().for_each(|&x| {
        reg.insert(x, ((x%100) as i32, ((x + 50) % 100) as i32));
    });

    read_bench.bench_with_input(
        BenchmarkId::new("Flat masks short siff, 40 frames registered", 
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.sum_rois_flim_flat(&masks.view(), frames, Some(&reg)).unwrap()))
        },
    );

    read_bench.bench_with_input(
        BenchmarkId::new("Flat masks individually short siff, 40 frames registered", 
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(||
                black_box(
                    masks.axis_iter(Axis(0)).for_each(|mask| {
                        siffreader.sum_roi_flim_flat(&mask, frames, Some(&reg)).unwrap();
                    })
                )
            )
        },
    );

    let siffreader = corrosiff::open_siff(LONG_SIFF_PATH).unwrap();
    let frame_vec = Vec::<u64>::from_iter(0..49999);

    let frame_dims = siffreader.image_dims().unwrap().to_tuple();
    let mut masks = Array3::<bool>::from_elem(
        (10, frame_dims.0, frame_dims.1), true
    );

    masks.iter_mut().for_each(|x| *x = rand::random::<bool>());

    read_bench.sample_size(10);
    read_bench.bench_with_input(
        BenchmarkId::new("Flat masks long siff, 50k-1 frames unregistered", 
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.sum_rois_flim_flat(&masks.view(), frames, None).unwrap()))
        },
    );

    read_bench.bench_with_input(
        BenchmarkId::new("Flat masks individually long siff, 50k-1 frames unregistered", 
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(||
                black_box(
                    masks.axis_iter(Axis(0)).for_each(|mask| {
                        siffreader.sum_roi_flim_flat(&mask, frames, None).unwrap();
                    })
                )
            )
        },
    );

    let mut reg : HashMap<u64, (i32, i32)> = HashMap::new();
    
    frame_vec.iter().for_each(|&x| {
        reg.insert(x, ((x % 100) as i32, ((x + 50) % 100) as i32 ));
    });

    read_bench.bench_with_input(
        BenchmarkId::new("Flat masks long siff, 50k-1 frames registered", 
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.sum_rois_flim_flat(&masks.view(), frames, Some(&reg)).unwrap()))
        },
    );

    read_bench.bench_with_input(
        BenchmarkId::new("Flat masks individually long siff, 50k-1 frames registered", 
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(||
                black_box(
                    masks.axis_iter(Axis(0)).for_each(|mask| {
                        siffreader.sum_roi_flim_flat(&mask, frames, Some(&reg)).unwrap();
                    })
                )
            )
        },
    );

    /////////////////// 3d masks ///////////////////////
    
    let siffreader = corrosiff::open_siff(SHORT_SIFF_PATH).unwrap();
    let frame_vec = Vec::<u64>::from_iter(0..40);

    let frame_dims = siffreader.image_dims().unwrap().to_tuple();
    let mut masks = Array4::<bool>::from_elem(
        (7, 10, frame_dims.0, frame_dims.1), true);

    masks.mapv_inplace(|x| rand::random::<bool>());

    read_bench.bench_with_input(
        BenchmarkId::new("Volume masks short siff, 40 frames unregistered", 
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.sum_rois_flim_volume(&masks.view(), frames, None).unwrap()))
        },
    );

    let mut reg : HashMap<u64, (i32, i32)> = HashMap::new();
    
    frame_vec.iter().for_each(|&x| {
        reg.insert(x, ((x%123) as i32, ((x + 50) % 87) as i32));
    });

    read_bench.bench_with_input(
        BenchmarkId::new("Volume masks short siff, 40 frames registered", 
            40,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.sum_rois_flim_volume(&masks.view(), frames, Some(&reg)).unwrap()))
        },
    );

    let siffreader = corrosiff::open_siff(LONG_SIFF_PATH).unwrap();
    let frame_vec = Vec::<u64>::from_iter(0..49999);

    let frame_dims = siffreader.image_dims().unwrap().to_tuple();
    let mut masks = Array4::<bool>::from_elem(
        (7, 10, frame_dims.0, frame_dims.1), true);


    masks.mapv_inplace(|x| rand::random::<bool>());

    read_bench.sample_size(10);
    read_bench.bench_with_input(
        BenchmarkId::new("Volume masks long siff, 50k-1 frames unregistered", 
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.sum_rois_flim_volume(&masks.view(), frames, None).unwrap()))
        },
    );

    let mut reg : HashMap<u64, (i32, i32)> = HashMap::new();
    
    frame_vec.iter().for_each(|&x| {
        reg.insert(x, ((x % 123) as i32, ((x + 50) % 187) as i32 ));
    });

    read_bench.bench_with_input(
        BenchmarkId::new("Volume masks long siff, 50k-1 frames registered", 
            49999,
        ),
        &frame_vec.as_slice(),
        |bench, frames| {
            bench.iter(|| black_box(siffreader.sum_rois_flim_volume(&masks.view(), frames, Some(&reg)).unwrap()))
        },
    );
}

// fn criterion_benchmark_histograms(c: &mut Criterion) {
//     let siffreader = corrosiff::open_siff(SHORT_SIFF_PATH).unwrap();
//     let mut read_bench = c.benchmark_group("Frame read benchmarks");
//     let frame_vec = Vec::<u64>::from_iter(0..40);
//     read_bench.bench_with_input(
//         BenchmarkId::new("Read histogram from 40 frames", 
//             40,
//         ),
//         &frame_vec.as_slice(),
//         |bench, frames| {
//             bench.iter(|| black_box(siffreader.get_histogram(frames).unwrap()))
//         },
//     );

//     let siffreader = corrosiff::open_siff(LONG_SIFF_PATH).unwrap();
//     let frame_vec = Vec::<u64>::from_iter(0..siffreader.num_frames() as u64);
//     read_bench.sample_size(20);
//     read_bench.bench_with_input(
//         BenchmarkId::new("Read long siff, get histogram from all frames", 
//             -1,
//         ),
//         &frame_vec.as_slice(),
//         |bench, frames| {
//             bench.iter(|| black_box(siffreader.get_histogram(frames).unwrap()))
//         },
//     );
// }

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = criterion_benchmark_read_one_mask,
    criterion_benchmark_read_multiple_masks,
    //criterion_benchmark_histograms,
);
criterion_main!(benches);