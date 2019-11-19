use crate::frame::{Frames, UnresolvedFrames};
use crate::report::{Report, ReportReader};
use std::collections::HashMap;
use std::sync::{RwLock, Arc};
use crate::channel::{bounded, Sender, Receiver};

use crossbeam::queue::ArrayQueue;
use crossbeam::sync::Parker;
use std::sync::atomic::Ordering;
use backtrace::Frame;
use crate::MAX_DEPTH;

lazy_static::lazy_static! {
    pub(crate) static ref COLLECTOR: RwLock<Collector> = RwLock::new(Collector::default());
}

#[derive(Default)]
pub struct Collector {
    backtrace_counter: HashMap<UnresolvedFrames, usize>,
    ptr_map: HashMap<u64, (UnresolvedFrames, usize)>, // Map ptr to frames and original size
}

impl Collector {
    pub fn init(&self) {
        println!("MEM RECORD START")
    }

    pub fn alloc(&mut self, addr: u64, size: usize, backtrace: UnresolvedFrames) {
        match self.backtrace_counter.get_mut(&backtrace) {
            Some(s) => {
                *s += size;
            }
            None => {
                self.backtrace_counter.insert(backtrace.clone(), size);
            }
        }

        match self
            .ptr_map
            .insert(addr, (backtrace.clone(), size)) {
            Some((frames, size)) => {
                println!("WARN! DUPLICATE ALLOC: {} {}", Frames::from(frames.clone()), size);
            }
            None => {

            }
        }
    }

    pub fn dealloc(&mut self, addr: u64, backtrace: UnresolvedFrames) {
        match self.ptr_map.get(&addr) {
            Some((bt, s)) => {
                match self.backtrace_counter.get_mut(bt) {
                    Some(size) => *size -= s,
                    None => {
                        println!("WARN UNRECORDED DEALLOC")
                    }
                }

                let complete_backtrace = UnresolvedFrames::new(
                    &bt.frames
                        .clone()
                        .into_iter()
                        .chain(backtrace.frames)
                        .collect::<Vec<Frame>>(),
                );

                self.backtrace_counter.insert(complete_backtrace, *s);
            }
            None => {
                println!("WARN UNRECORDED DEALLOC")
            }
        };

        match self.ptr_map.remove(&addr) {
            Some(_) => {},
            None => {println!("WARN UNRECORDED DEALLOC")}
        }
    }

    pub fn report(&self) -> Report {
        Report {
            data: self
                .backtrace_counter
                .iter()
                .map(|(frames, size)| (Frames::from(frames.clone()), *size))
                .collect(),
        }
    }
}

enum Operation {
    Alloc(u64, usize, ([Frame; MAX_DEPTH], usize)),
    Dealloc(u64, ([Frame; MAX_DEPTH], usize)),
    DropReport(Report),
    Report,
}

pub struct CollectorClient {
    operation_sender: Sender<Operation>,
    report_receiver: Receiver<Report>,
}

impl Default for CollectorClient {
    fn default() -> Self {
        let mut collector = Collector::default();
        let (operation_sender, operation_receiver) = bounded(1);
        let (report_sender, report_receiver) = bounded(1);

        let mut p = Parker::new();
        let u = p.unparker().clone();

        std::thread::Builder::new()
            .name("collector".to_owned())
            .spawn(move || {
                use crate::profiler::PROFILE;

                PROFILE.with(|profile| {
                    profile.store(false, Ordering::SeqCst);
                });

                u.unpark();
                loop {
                    match operation_receiver.recv() {
                        Operation::Alloc(ptr, size, (frames, depth)) => {
                            collector.alloc(ptr, size, UnresolvedFrames::new(&frames[0..depth]))
                        }
                        Operation::Dealloc(ptr, (frames, depth)) => {
                            collector.dealloc(ptr, UnresolvedFrames::new(&frames[0..depth]))
                        }
                        Operation::Report => {
                            report_sender.send(collector.report());
                        }
                        Operation::DropReport(report) => {
                            drop(report)
                        }
                    }
                }
        }).unwrap();

        p.park();

        CollectorClient {
            operation_sender,
            report_receiver,
        }
    }
}

impl CollectorClient {
    pub fn alloc(&self, addr: u64, size: usize, backtrace: ([Frame; MAX_DEPTH], usize)) {
        self.operation_sender.send(Operation::Alloc(addr, size, backtrace));
    }

    pub fn dealloc(&self, addr: u64, backtrace: ([Frame; MAX_DEPTH], usize)) {
        self.operation_sender.send(Operation::Dealloc(addr, backtrace));
    }

    pub fn drop_report(&self, report: Report) {
        self.operation_sender.send(Operation::DropReport(report));
    }

    pub fn report(&self) -> ReportReader {
        self.operation_sender.send(Operation::Report);

        let report = self.report_receiver.recv();
        let report_reader = ReportReader::new(report, self);

        report_reader
    }
}