#![feature(const_fn)]

mod collector;
mod frame;
mod report;

use std::alloc::{GlobalAlloc, Layout};

use frame::UnresolvedFrames;
use report::Report;

use crate::collector::Collector;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::RwLock;

fn get_backtrace() -> UnresolvedFrames {
    let mut skip = 3;

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

pub struct AllocRecorder<T: GlobalAlloc> {
    pub inner: T,
    pub record: AtomicBool,
    pub collector: AtomicPtr<RwLock<Collector>>,
}

impl<T: GlobalAlloc> AllocRecorder<T> {
    pub const fn new(inner: T) -> AllocRecorder<T> {
        AllocRecorder {
            inner,
            record: AtomicBool::new(false),
            collector: AtomicPtr::new(null_mut()),
        }
    }

    pub fn flush(&self) {
        let ptr = self.collector.load(Ordering::SeqCst);
        if !ptr.is_null() {
            let _collector = unsafe { Box::from_raw(ptr) };
        }

        let collector = Box::new(RwLock::new(Collector::default()));
        self.collector.store(
            Box::leak(collector) as *mut RwLock<Collector>,
            Ordering::SeqCst,
        );
    }

    pub fn start_record(&self) {
        self.record.store(true, Ordering::SeqCst);
    }

    pub fn stop_record(&self) {
        self.record.store(false, Ordering::SeqCst);
    }

    pub fn report(&self) -> Report {
        self.stop_record();

        let report = unsafe {
            (*self.collector.load(Ordering::SeqCst))
                .read()
                .unwrap()
                .report()
        };

        self.start_record();
        report
    }
}

unsafe impl<T: GlobalAlloc> GlobalAlloc for AllocRecorder<T> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self.inner.alloc(layout);
        if self.record.load(Ordering::SeqCst) {
            self.stop_record();

            let collector = &*self.collector.load(Ordering::SeqCst);
            match collector.write() {
                Ok(mut guard) => {
                    guard.alloc(std::mem::transmute(ptr), layout.size(), get_backtrace());
                }
                Err(_) => {
                    unreachable!();
                }
            }

            self.start_record();
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner.dealloc(ptr, layout);

        if self.record.load(Ordering::SeqCst) {
            self.stop_record();

            let collector = &*self.collector.load(Ordering::SeqCst);
            match collector.write() {
                Ok(mut guard) => {
                    guard.dealloc(std::mem::transmute(ptr), get_backtrace());
                }
                Err(_) => {
                    unreachable!();
                }
            }

            self.start_record();
        }
    }
}
