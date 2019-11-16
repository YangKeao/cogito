use cogito::AllocRecorder;
use std::alloc::System;
use std::fs::File;
use std::sync::atomic::Ordering;

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
    ALLOC.init_collector();

    let mut vec = Vec::new();

    for _ in 0..100 {
        vec.push(rand::random());
    }

    let mut thread_handlers = Vec::new();
    for _ in 0..10 {
        let vec = vec.clone();
        thread_handlers.push(std::thread::spawn(move || {
            let _sorted = quick_sort(vec);
        }));
    }

    for thread in thread_handlers {
        thread.join().unwrap();
    }

    let report = ALLOC.report();

    let file = File::create("flamegraph.svg").unwrap();
    report.flamegraph(file);

    println!("report: {}", &report);

    cogito::PROFILE.with(|profile| {
        profile.store(false, Ordering::SeqCst);
    })
}
