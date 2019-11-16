use backtrace::Frame;
use rustc_demangle::demangle;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::os::raw::c_void;
use std::path::PathBuf;

use crate::MAX_DEPTH;

#[derive(Clone)]
pub struct UnresolvedFrames {
    pub frames: Vec<Frame>,
}

impl UnresolvedFrames {
    pub fn new(bt: &[Frame]) -> Self {
        Self { frames: bt.to_vec() }
    }
}

impl PartialEq for UnresolvedFrames {
    fn eq(&self, other: &Self) -> bool {
        if self.frames.len() == other.frames.len() {
            let iter = self.frames.iter().zip(other.frames.iter());

            iter.map(|(self_frame, other_frame)| {
                self_frame.symbol_address() == other_frame.symbol_address()
            })
            .all(|result| result)
        } else {
            false
        }
    }
}

impl Eq for UnresolvedFrames {}

impl Hash for UnresolvedFrames {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.frames
            .iter()
            .for_each(|frame| frame.symbol_address().hash(state));
    }
}

/// Symbol is a representation of a function symbol. It contains name and addr of it. If built with
/// debug message, it can also provide line number and filename. The name in it is not demangled.
#[derive(Debug, Clone)]
pub struct Symbol {
    /// This name is raw name of a symbol (which hasn't been demangled).
    pub name: Option<Vec<u8>>,

    /// The address of the function. It is not 100% trustworthy.
    pub addr: Option<*mut c_void>,

    /// Line number of this symbol. If compiled with debug message, you can get it.
    pub lineno: Option<u32>,

    /// Filename of this symbol. If compiled with debug message, you can get it.
    pub filename: Option<PathBuf>,
}

impl Symbol {
    pub fn name(&self) -> String {
        match &self.name {
            Some(name) => match std::str::from_utf8(&name) {
                Ok(name) => format!("{}", demangle(name)),
                Err(_) => "NonUtf8Name".to_owned(),
            },
            None => "Unknown".to_owned(),
        }
    }

    pub fn sys_name(&self) -> &str {
        match &self.name {
            Some(name) => match std::str::from_utf8(&name) {
                Ok(name) => name,
                Err(_) => "NonUtf8Name",
            },
            None => "Unknown",
        }
    }

    pub fn filename(&self) -> &str {
        match &self.filename {
            Some(name) => match name.as_os_str().to_str() {
                Some(name) => name,
                None => "NonUtf8Name",
            },
            None => "Unknown",
        }
    }

    pub fn lineno(&self) -> u32 {
        self.lineno.unwrap_or(0)
    }
}

unsafe impl Send for Symbol {}

impl From<&backtrace::Symbol> for Symbol {
    fn from(symbol: &backtrace::Symbol) -> Self {
        Symbol {
            name: symbol
                .name()
                .and_then(|name| Some(name.as_bytes().to_vec())),
            addr: symbol.addr(),
            lineno: symbol.lineno(),
            filename: symbol
                .filename()
                .and_then(|filename| Some(filename.to_owned())),
        }
    }
}

impl Display for Symbol {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match &self.name {
            Some(name) => match std::str::from_utf8(&name) {
                Ok(name) => write!(f, "{}", demangle(name))?,
                Err(_) => write!(f, "NonUtf8Name")?,
            },
            None => {
                write!(f, "Unknown")?;
            }
        }
        match &self.filename {
            Some(filename) => write!(f, ":{:?}", filename)?,
            None => {}
        }
        match &self.lineno {
            Some(lineno) => write!(f,":{}", lineno)?,
            None => {}
        }
        Ok(())
    }
}

impl PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        match &self.name {
            Some(name) => match &other.name {
                Some(other_name) => name == other_name,
                None => false,
            },
            None => other.name.is_none(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Frames {
    pub frames: Vec<Vec<Symbol>>,
}

impl From<UnresolvedFrames> for Frames {
    fn from(frames: UnresolvedFrames) -> Self {
        let mut fs = Vec::new();
        frames.frames.iter().for_each(|frame| {
            let mut symbols = Vec::new();
            backtrace::resolve_frame(frame, |symbol| {
                symbols.push(Symbol::from(symbol));
            });
            fs.push(symbols);
        });

        Self { frames: fs }
    }
}

impl PartialEq for Frames {
    fn eq(&self, other: &Self) -> bool {
        if self.frames.len() == other.frames.len() {
            let iter = self.frames.iter().zip(other.frames.iter());

            iter.map(|(self_frame, other_frame)| {
                if self_frame.len() == other_frame.len() {
                    let iter = self_frame.iter().zip(other_frame.iter());
                    iter.map(|(self_symbol, other_symbol)| self_symbol == other_symbol)
                        .all(|result| result)
                } else {
                    false
                }
            })
            .all(|result| result)
        } else {
            false
        }
    }
}

impl Eq for Frames {}

impl Hash for Frames {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.frames.iter().for_each(|frame| {
            frame.iter().for_each(|symbol| match &symbol.name {
                Some(name) => name.hash(state),
                None => 0.hash(state),
            })
        });
    }
}

impl Display for Frames {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        for frame in self.frames.iter() {
            write!(f, "FRAME: ")?;
            for symbol in frame.iter() {
                write!(f, "{} -> ", symbol)?;
            }
        }

        Ok(())
    }
}
