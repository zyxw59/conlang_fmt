use std::fmt;
use std::io::{Result as IoResult, Write};

use crate::document::Document;
use crate::errors::Result as EResult;
use crate::text::Referenceable;

pub mod contents;
pub mod control;
pub mod gloss;
pub mod heading;
pub mod list;
pub mod replacements;
pub mod table;

use control::DocumentControl;
use gloss::Gloss;
use heading::HeadingLike;
use replacements::Replacements;
use table::Table;

#[cfg(test)]
use list::List;

type OResult<T> = EResult<Option<T>>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Parameter(pub Option<String>, pub String);

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(key) = &self.0 {
            write!(f, "{key}={}", self.1)
        } else {
            f.write_str(&self.1)
        }
    }
}

pub trait UpdateParam {
    /// Updates with the given parameter. If the parameter was not updated, returns the parameter.
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter>;
}

impl UpdateParam for String {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref().map(|n| n.as_ref()) {
            Some("class") | None => {
                *self = param.1;
                None
            }
            _ => Some(param),
        })
    }
}

#[derive(Debug)]
pub struct Block {
    pub kind: Box<dyn BlockType>,
    pub common: BlockCommon,
}

impl UpdateParam for Block {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        self.kind.update_param(param).and_then(|p| match p {
            Some(p) => self.common.update_param(p),
            None => Ok(None),
        })
    }
}

impl<T: BlockType + 'static> From<T> for Block {
    fn from(kind: T) -> Block {
        Block {
            kind: Box::new(kind),
            common: Default::default(),
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct BlockCommon {
    pub class: String,
    pub id: String,
    pub start_line: usize,
}

impl BlockCommon {
    pub fn new(start_line: usize) -> BlockCommon {
        BlockCommon {
            start_line,
            ..Default::default()
        }
    }
}

impl UpdateParam for BlockCommon {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref().map(|n| n.as_ref()) {
            Some("class") | None => {
                self.class = param.1;
                None
            }
            Some("id") => {
                self.id = param.1;
                None
            }
            _ => Some(param),
        })
    }
}

pub trait BlockType: fmt::Debug {
    /// Outputs the block.
    fn write(&self, w: &mut dyn Write, common: &BlockCommon, document: &Document) -> IoResult<()>;

    /// Updates with the given parameter. If the parameter was not updated, returns the parameter.
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(Some(param))
    }

    /// Returns a `&dyn Referenceable` if the block can be referenced, otherwise returns `None`.
    fn as_referenceable(&self) -> Option<&dyn Referenceable> {
        None
    }

    /// Returns a `&dyn HeadingLike` if the block is a heading, otherwise returns `None`.
    fn as_heading(&self) -> Option<&dyn HeadingLike> {
        None
    }

    /// Returns a `&mut dyn HeadingLike` if the block is a heading, otherwise returns `None`.
    fn as_mut_heading(&mut self) -> Option<&mut dyn HeadingLike> {
        None
    }

    /// Returns a `Replacements` if the block is a replacements block, otherwise returns `None`.
    fn as_mut_replacements(&mut self) -> Option<&mut Replacements> {
        None
    }

    #[cfg(test)]
    fn as_list(&self) -> Option<&List> {
        None
    }

    /// Returns a `&mut Table` if the block is a table, otherwise returns `None`.
    fn as_mut_table(&mut self) -> Option<&mut Table> {
        None
    }

    /// Returns a `&mut Table` if the block is a table, otherwise returns `None`.
    fn as_mut_gloss(&mut self) -> Option<&mut Gloss> {
        None
    }

    /// Returns a `&DocumentControl` if the block is a document control block, otherwise returns `None`.
    fn as_control(&self) -> Option<&DocumentControl> {
        None
    }
}

impl<T: BlockType> UpdateParam for T {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        BlockType::update_param(self, param)
    }
}
