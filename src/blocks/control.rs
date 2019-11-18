use std::io::{Result as IoResult, Write};

use crate::blocks::{BlockCommon, BlockType};
use crate::document::Document;
use crate::text::Text;

#[derive(Debug, Eq, PartialEq)]
pub enum DocumentControl {
    Title(Text),
    Stylesheet(Text),
    Author(Text),
    Description(Text),
    Lang(Text),
}

impl BlockType for DocumentControl {
    fn write(&self, _: &mut dyn Write, _: &BlockCommon, _: &Document) -> IoResult<()> {
        Ok(())
    }

    fn as_control(&self) -> Option<&DocumentControl> {
        Some(self)
    }
}
