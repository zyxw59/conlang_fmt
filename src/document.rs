use std::collections::HashMap;
use std::default::Default;

use failure::ResultExt;

use errors::{ErrorKind, Result as EResult};

type OResult<T> = EResult<Option<T>>;

#[derive(Debug, Default)]
pub struct Document {
    /// A list of blocks in the document
    blocks: Vec<Block>,
    /// A list of indices into the `blocks` field corresponding to the top-level section headings
    /// of the document.
    sections: Vec<usize>,
    /// A list of indices into the `blocks` field corresponding to the tables of the document.
    tables: Vec<usize>,
    /// A list of indices into the `blocks` field corresponding to the glosses of the document.
    glosses: Vec<usize>,
    /// A map from IDs to indices into the `blocks` field.
    ids: HashMap<String, usize>,
}

pub struct Parameter(pub Option<String>, pub String);

#[derive(Debug)]
pub struct Block {
    pub kind: BlockType,
    pub common: BlockCommon,
}

impl Block {
    pub fn heading() -> Block {
        Block {
            kind: BlockType::Heading(Default::default()),
            common: Default::default(),
        }
    }

    pub fn contents() -> Block {
        Block {
            kind: BlockType::Contents(Default::default()),
            common: Default::default(),
        }
    }

    pub fn list() -> Block {
        Block {
            kind: BlockType::List(Default::default()),
            common: Default::default(),
        }
    }

    pub fn table() -> Block {
        Block {
            kind: BlockType::Table(Default::default()),
            common: Default::default(),
        }
    }

    pub fn gloss() -> Block {
        Block {
            kind: BlockType::Gloss(Default::default()),
            common: Default::default(),
        }
    }

    pub fn paragraph() -> Block {
        Block {
            kind: BlockType::Paragraph(Default::default()),
            common: Default::default(),
        }
    }

    /// Updates with the given parameter. If the parameter was not updated, returns the parameter.
    pub fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        self.kind.update_param(param).and_then(|p| match p {
            Some(p) => self.common.update_param(p),
            None => Ok(None),
        })
    }
}

#[derive(Debug, Default)]
pub struct BlockCommon {
    pub class: String,
    pub id: String,
}

impl BlockCommon {
    /// Updates with the given parameter. If the parameter was not updated, returns the parameter.
    pub fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
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

#[derive(Debug)]
pub enum BlockType {
    Heading(Heading),
    Contents(Contents),
    List(List),
    Table(Table),
    Gloss(Gloss),
    Paragraph(Text),
}

impl BlockType {
    /// Updates with the given parameter. If the parameter was not updated, returns the parameter.
    pub fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        match *self {
            BlockType::Heading(ref mut heading) => heading.update_param(param),
            BlockType::Contents(ref mut contents) => contents.update_param(param),
            BlockType::List(ref mut list) => list.update_param(param),
            BlockType::Table(ref mut table) => table.update_param(param),
            BlockType::Gloss(ref mut gloss) => gloss.update_param(param),
            BlockType::Paragraph(_) => Ok(Some(param)),
        }
    }
}

#[derive(Debug)]
pub struct Heading {
    pub title: Text,
    pub numbered: bool,
    pub toc: bool,
    pub level: usize,
    pub children: Vec<usize>,
}

impl Heading {
    /// Updates with the given parameter. If the parameter was not updated, returns the parameter.
    pub fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref() {
            Some(_) => Some(param),
            None => match param.1.as_ref() {
                "nonumber" => {
                    self.numbered = false;
                    None
                }
                "notoc" => {
                    self.toc = false;
                    None
                }
                _ => Some(param),
            },
        })
    }
}

impl Default for Heading {
    fn default() -> Heading {
        Heading {
            title: Default::default(),
            numbered: true,
            toc: true,
            level: Default::default(),
            children: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct Contents {
    pub title: Text,
    pub max_level: usize,
}

impl Contents {
    /// Updates with the given parameter. If the parameter was not updated, returns the parameter.
    pub fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref().map(|n| n.as_ref()) {
            Some("max_level") => {
                self.max_level = param.1.parse::<usize>().with_context(|_| ErrorKind::Parse)?;
                None
            }
            _ => Some(param),
        })
    }
}

impl Default for Contents {
    fn default() -> Contents {
        Contents {
            title: Text::from("Table of Contents"),
            max_level: 6,
        }
    }
}

#[derive(Debug, Default)]
pub struct List {
    pub items: Vec<ListItem>,
    pub ordered: bool,
}

impl List {
    /// Updates with the given parameter. If the parameter was not updated, returns the parameter.
    pub fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref() {
            Some(_) => Some(param),
            None => match param.1.as_ref() {
                "ordered" => {
                    self.ordered = true;
                    None
                }
                _ => Some(param),
            },
        })
    }
}

#[derive(Debug, Default)]
pub struct ListItem {
    pub text: Text,
    pub sublist: Option<List>,
}

#[derive(Debug)]
pub struct Table {
    pub title: Text,
    pub numbered: bool,
    pub rows: Vec<Row>,
    pub columns: Vec<Column>,
}

impl Table {
    /// Updates with the given parameter. If the parameter was not updated, returns the parameter.
    pub fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref() {
            Some(_) => Some(param),
            None => match param.1.as_ref() {
                "nonumber" => {
                    self.numbered = false;
                    None
                }
                _ => Some(param),
            },
        })
    }
}

impl Default for Table {
    fn default() -> Table {
        Table {
            title: Default::default(),
            numbered: true,
            rows: Default::default(),
            columns: Default::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Row {
    pub cells: Vec<Cell>,
    pub header: bool,
    pub class: String,
}

#[derive(Debug, Default)]
pub struct Column {
    pub header: bool,
    pub class: String,
}

#[derive(Debug, Default)]
pub struct Cell {
    rows: usize,
    cols: usize,
    text: Text,
}

#[derive(Debug)]
pub struct Gloss {
    pub title: Text,
    pub numbered: bool,
    pub preamble: Vec<Text>,
    pub gloss: Vec<Vec<Text>>,
    pub postamble: Vec<Text>,
}

impl Gloss {
    /// Updates with the given parameter. If the parameter was not updated, returns the parameter.
    pub fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref() {
            Some(_) => Some(param),
            None => match param.1.as_ref() {
                "nonumber" => {
                    self.numbered = false;
                    None
                }
                _ => Some(param),
            },
        })
    }
}

impl Default for Gloss {
    fn default() -> Gloss {
        Gloss {
            title: Default::default(),
            numbered: true,
            preamble: Default::default(),
            gloss: Default::default(),
            postamble: Default::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Text(pub Vec<Inline>);

impl<'a> From<&'a str> for Text {
    fn from(s: &'a str) -> Text {
        Text(vec![s.into()])
    }
}

#[derive(Debug)]
pub struct Inline {
    pub kind: InlineType,
    pub class: String,
}

impl<'a> From<&'a str> for Inline {
    fn from(s: &'a str) -> Inline {
        Inline {
            kind: InlineType::Text(s.into()),
            class: Default::default(),
        }
    }
}

#[derive(Debug)]
pub enum InlineType {
    Emphasis(Text),
    Strong(Text),
    Italics(Text),
    Bold(Text),
    SmallCaps(Text),
    Span(Text),
    Replace(String),
    Reference(String),
    Link(Link),
    Text(String),
}

#[derive(Debug, Default)]
pub struct Link {
    pub url: String,
    pub title: Text,
}
