#![feature(const_fn)]

mod collector;
mod frame;
mod report;
mod profiler;
mod channel;

pub const MAX_DEPTH: usize = 128;

pub use profiler::*;