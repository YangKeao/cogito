use crate::frame::Frames;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use crate::collector::CollectorClient;

pub struct Report {
    pub data: HashMap<Frames, usize>,
}

pub struct ReportReader<'a> {
    inner_report: Option<Report>,
    client: &'a CollectorClient
}

impl<'a> Drop for ReportReader<'a> {
    fn drop(&mut self) {
        let report = self.inner_report.take().unwrap();
        self.client.drop_report(report);
    }
}

impl<'a> AsRef<Report> for ReportReader<'a> {
    fn as_ref(&self) -> &Report {
        if let Some(inner) = &self.inner_report {
            inner
        } else {
            unreachable!()
        }
    }
}

impl<'a> ReportReader<'a> {
    pub fn new(inner: Report, client: &'a CollectorClient) -> ReportReader<'a> {
        Self { inner_report: Some(inner), client,}
    }
}

impl Display for Report {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        for (key, val) in self.data.iter() {
            write!(f, "{} {}", key, val)?;
            writeln!(f)?;
        }

        Ok(())
    }
}

mod flamegraph {
    use super::*;
    use std::io::Write;

    impl Report {
        pub fn flamegraph<W>(&self, writer: W)
        where
            W: Write,
        {
            use inferno::flamegraph;

            let lines: Vec<String> = self
                .data
                .iter()
                .map(|(key, value)| {
                    let mut line = String::new();

                    for frame in key.frames.iter().rev() {
                        for symbol in frame.iter().rev() {
                            line.push_str(&format!("{}/", symbol));
                        }
                        line.pop().unwrap_or_default();
                        line.push(';');
                    }

                    line.pop().unwrap_or_default();
                    line.push_str(&format!(" {}", value));

                    line
                })
                .collect();
            if !lines.is_empty() {
                let mut options = flamegraph::Options::default();
                options.hash = true;
                options.count_name = "bytes".to_owned();

                flamegraph::from_lines(&mut options, lines.iter().map(|s| &**s), writer).unwrap(); // TODO: handle this error
            }
        }
    }
}
