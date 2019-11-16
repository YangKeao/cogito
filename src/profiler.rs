use std::alloc::{GlobalAlloc, Layout};

use crate::frame::UnresolvedFrames;
use crate::report::{Report, ReportReader};
use crate::collector::{Collector, CollectorClient};

use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::RwLock;

use crate::MAX_DEPTH;
use backtrace::Frame;

thread_local! {
    pub static PROFILE: AtomicBool = AtomicBool::new(true);
}

fn get_backtrace() -> ([Frame; MAX_DEPTH], usize) {
    let mut skip = 3;

    let mut bt: [Frame; MAX_DEPTH] = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
    let mut index = 0;

    backtrace::trace(|frame| {
        if skip > 0 {
            skip -= 1;
            true
        } else {
            if index < MAX_DEPTH {
                bt[index] = frame.clone();
                index += 1;
                true
            } else {
                false
            }
        }
    });

    (bt, index)
}

pub struct AllocRecorder<T: GlobalAlloc> {
    pub inner: T,
    pub collector: AtomicPtr<CollectorClient>,
}

impl<T: GlobalAlloc> AllocRecorder<T> {
    pub const fn new(inner: T) -> AllocRecorder<T> {
        AllocRecorder {
            inner,
            collector: AtomicPtr::new(null_mut()),
        }
    }

    pub fn init_collector(&self) {
        let collector = Box::new(CollectorClient::default());

        self.collector.store(Box::leak(collector), Ordering::SeqCst);
    }

    pub fn report(&self) -> ReportReader {
        let report = unsafe {
            (*self.collector.load(Ordering::SeqCst))
                .report()
        };

        report
    }
}

unsafe impl<T: GlobalAlloc> GlobalAlloc for AllocRecorder<T> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self.inner.alloc(layout);

        let addr = std::mem::transmute(ptr);
        PROFILE.with(move |profile| {
            if profile.load(Ordering::SeqCst) {
                let collector = self.collector.load(Ordering::SeqCst);
                if !collector.is_null() {
                    let collector = &*collector;
                    collector.alloc(addr , layout.size(), get_backtrace());
                }
            }
        });

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let addr = std::mem::transmute(ptr);
        PROFILE.with(move |profile| {
            if profile.load(Ordering::SeqCst) {
                let collector = self.collector.load(Ordering::SeqCst);
                if !collector.is_null() {
                    let collector = &*collector;
                    collector.dealloc(addr , get_backtrace());
                }
            }
        });

        self.inner.dealloc(ptr, layout);
    }
}
