use crate::frame::{Frames, UnresolvedFrames};
use crate::report::Report;
use std::collections::HashMap;
use std::sync::RwLock;

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

        if let Some(_) = self.ptr_map.insert(addr, (backtrace.clone(), size)) {
            unreachable!();
        }
    }

    pub fn free(&mut self, addr: u64, backtrace: UnresolvedFrames) {
        match self.ptr_map.get(&addr) {
            Some((bt, s)) => {
                match self.backtrace_counter.get_mut(bt) {
                    Some(size) => *size -= s,
                    None => {
                        unreachable!();
                    }
                }

                let complete_backtrace = UnresolvedFrames::new(
                    bt.frames
                        .clone()
                        .into_iter()
                        .chain(backtrace.frames)
                        .collect(),
                );

                self.backtrace_counter.insert(complete_backtrace, *s);
            }
            None => {
                unreachable!();
            }
        };

        self.ptr_map.remove(&addr).unwrap();
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
