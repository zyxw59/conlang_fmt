use std::fmt;
use std::io;
use std::result;

use failure::{Backtrace, Context, Fail};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        self.inner.get_context()
    }
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }
}

#[derive(Clone, Debug, Eq, Fail, PartialEq)]
pub enum ErrorKind {
    #[fail(display = "Failed to parse block starting on line {}", _0)]
    Block(usize),
    #[fail(display = "Unexpected end of block, {}", _0)]
    EndOfBlock(EndOfBlockKind),
    #[fail(display = "Parsing error")]
    Parse,
    #[fail(display = "Unknown parameter {}", _0)]
    Parameter(String),
    #[fail(display = "Invalid UTF-8 in line {}", _0)]
    Unicode(usize),
    #[fail(display = "An IO error occurred while reading line {}", _0)]
    Io(usize),
}

impl ErrorKind {
    pub fn from_io(err: &io::Error, line: usize) -> ErrorKind {
        match err.kind() {
            io::ErrorKind::InvalidData => ErrorKind::Unicode(line),
            _ => ErrorKind::Io(line),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Fail, PartialEq)]
pub enum EndOfBlockKind {
    #[fail(display = "expected a character after `\\`")]
    Escape,
    #[fail(display = "expected `{}`", _0)]
    Expect(char),
}
