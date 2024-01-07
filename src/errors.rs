use std::io;

pub use anyhow::{Error, Result};

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ErrorKind {
    #[error("Failed to parse block starting on line {0}")]
    Block(usize),
    #[error("Unexpected end of block, {0}")]
    EndOfBlock(EndOfBlockKind),
    #[error("Expected `{0}`, got `{1}`")]
    Expected(char, char),
    #[error("Gloss line after postamble")]
    GlossLine,
    #[error("Parsing error")]
    Parse,
    #[error("Unknown parameter {0}")]
    Parameter(String),
    #[error("Duplicate ID {0}")]
    Id(String),
    #[error("Duplicate replace directive {0}")]
    Replace(String),
    #[error("Invalid UTF-8 in line {0}")]
    Unicode(usize),
    #[error("An IO error occurred while reading line {0}")]
    ReadIo(usize),
    #[error("File {0} not found")]
    FileNotFound(String),
    #[error("An IO error occurred while writing block starting on line {0}")]
    WriteIo(usize),
    #[error("An IO error occurred while writing head matter")]
    WriteIoHead,
    #[error("An IO error occurred while writing tail matter")]
    WriteIoTail,
}

impl ErrorKind {
    pub fn input_error(err: io::Error, line: usize) -> Error {
        let context = match err.kind() {
            io::ErrorKind::InvalidData => ErrorKind::Unicode(line),
            _ => ErrorKind::ReadIo(line),
        };
        Error::new(err).context(context)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum EndOfBlockKind {
    #[error("expected a character after `\\`")]
    Escape,
    #[error("expected `{0}`")]
    Expect(char),
}
