use std::io::{Result as IoResult, Write};

use crate::blocks::{BlockCommon, BlockType};
use crate::document::Document;

#[derive(Debug, Eq, PartialEq)]
pub enum DocumentControl {
    Title(String),
    Stylesheet(String),
    Author(String),
    Description(String),
    Lang(String),
}

impl BlockType for DocumentControl {
    fn write(&self, _: &mut dyn Write, _: &BlockCommon, _: &Document) -> IoResult<()> {
        Ok(())
    }

    fn as_control(&self) -> Option<&DocumentControl> {
        Some(self)
    }
}
