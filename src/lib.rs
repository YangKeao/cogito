mod collector;
mod frame;
mod report;


use std::alloc::{GlobalAlloc, Layout, System};


use collector::COLLECTOR;
use frame::UnresolvedFrames;
use report::Report;

use std::sync::atomic::{AtomicBool, Ordering};

fn get_backtrace() -> UnresolvedFrames {
    let mut skip = 2;

    let mut bt = Vec::new();

    backtrace::trace(|frame| {
        if skip > 0 {
            skip -= 1;
        } else {
            bt.push(frame.clone());
        }
        true
    });

    UnresolvedFrames::new(bt)
}

struct MyAllocator {
    pub record: AtomicBool,
}

unsafe impl GlobalAlloc for MyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if self.record.load(Ordering::SeqCst) {
            match COLLECTOR.write() {
                Ok(mut guard) => {
                    self.record.store(false, Ordering::SeqCst);
                    guard.alloc(
                        std::mem::transmute(ptr),
                        layout.size(),
                        get_backtrace(),
                    );
                    self.record.store(true, Ordering::SeqCst);
                }
                Err(_) => {
                    unreachable!();
                }
            }
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);

        if self.record.load(Ordering::SeqCst) {
            match COLLECTOR.write() {
                Ok(mut guard) => {
                    self.record.store(false, Ordering::SeqCst);
                    guard.free(std::mem::transmute(ptr), get_backtrace());
                    self.record.store(true, Ordering::SeqCst);
                }
                Err(_) => {
                    unreachable!();
                }
            }
        }
    }
}

#[global_allocator]
static A: MyAllocator = MyAllocator {
    record: AtomicBool::new(false),
};

fn init() {
    COLLECTOR.read().unwrap().init();
}

pub fn start() {
    init();

    A.record.store(true, Ordering::SeqCst)
}

pub fn stop() {
    A.record.store(false, Ordering::SeqCst)
}

pub fn report() -> Report {
    COLLECTOR.read().unwrap().report()
}
