use std::fs::File;
use std::alloc::System;
use cogito::AllocRecorder;

#[global_allocator]
static ALLOC: AllocRecorder<System> = AllocRecorder::new(System);

fn quick_sort(input: Vec<u32>) -> Vec<u32> {
    if input.len() == 0 {
        return Vec::new();
    }

    let mid = input[0];
    if input.len() > 1 {
        let left: Vec<u32> = input
            .iter()
            .filter(|item| **item < mid)
            .map(|item| item.clone())
            .collect();
        let right: Vec<u32> = input
            .iter()
            .filter(|item| **item > mid)
            .map(|item| item.clone())
            .collect();

        quick_sort(left)
            .into_iter()
            .chain(vec![mid].into_iter())
            .chain((quick_sort(right)).into_iter())
            .collect()
    } else {
        vec![mid]
    }
}

fn main() {
    ALLOC.flush();
    ALLOC.start_record();

    let mut vec = Vec::new();

    for _ in 0..1000 {
        vec.push(rand::random());
    }

    let _sorted = quick_sort(vec);

    let report = ALLOC.report();
    ALLOC.stop_record();

    let file = File::create("flamegraph.svg").unwrap();
    report.flamegraph(file);

    println!("report: {}", &report);
}
