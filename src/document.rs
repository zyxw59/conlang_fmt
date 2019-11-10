use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::default::Default;
use std::fmt::Debug;

use failure::ResultExt;

use crate::errors::{ErrorKind, Result as EResult};

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

impl Document {
    /// Adds the given block to the document.
    pub fn add_block(&mut self, block: Block) -> EResult<()> {
        let idx = self.blocks.len();
        if let Some(&Heading { level, .. }) = block.kind.as_heading() {
            if level == 1 || self.sections.len() == 0 {
                self.sections.push(idx);
            } else {
                // get index into `blocks` of last section
                let mut curr = *self.sections.last().unwrap();
                loop {
                    // All blocks in the heading tree should be headings; there's no other
                    // chance for them to get added
                    let h = self.blocks[curr].kind.as_mut_heading().unwrap();
                    if h.level == level - 1 {
                        // add this section to its parent and break
                        h.children.push(idx);
                        break;
                    } else {
                        // get index into `blocks` of last subsection
                        curr = *h.children.last().unwrap();
                    }
                }
            }
        }
        if block.kind.is_table() {
            self.tables.push(idx);
        }
        if block.kind.is_gloss() {
            self.glosses.push(idx);
        }
        let id = block.common.id.clone();
        if !id.is_empty() {
            match self.ids.entry(id) {
                Entry::Occupied(e) => Err(ErrorKind::Id(e.key().clone()).into()),
                Entry::Vacant(e) => {
                    e.insert(idx);
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Parameter(pub Option<String>, pub String);

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

impl From<Text> for Block {
    fn from(t: Text) -> Block {
        Block {
            kind: Box::new(t),
            common: Default::default(),
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct BlockCommon {
    pub class: String,
    pub id: String,
}

impl BlockCommon {
    pub fn new() -> BlockCommon {
        Default::default()
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

pub trait BlockType: Debug {
    /// Updates with the given parameter. If the parameter was not updated, returns the parameter.
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(Some(param))
    }

    /// Returns a `&Heading` if the block is a heading, otherwise returns `None`.
    fn as_heading(&self) -> Option<&Heading> {
        None
    }

    /// Returns a `&mut Heading` if the block is a heading, otherwise returns `None`.
    fn as_mut_heading(&mut self) -> Option<&mut Heading> {
        None
    }

    #[cfg(test)]
    fn as_list(&self) -> Option<&List> {
        None
    }

    /// Returns whether the block should be included in the list of tables.
    fn is_table(&self) -> bool {
        false
    }

    /// Returns whether the block should be included in the list of glosses.
    fn is_gloss(&self) -> bool {
        false
    }
}

impl<T: BlockType> UpdateParam for T {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        BlockType::update_param(self, param)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Heading {
    pub title: Text,
    pub numbered: bool,
    pub toc: bool,
    pub level: usize,
    pub children: Vec<usize>,
}

impl Heading {
    pub fn new() -> Heading {
        Default::default()
    }
}

impl BlockType for Heading {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
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

    fn as_heading(&self) -> Option<&Heading> {
        Some(self)
    }

    fn as_mut_heading(&mut self) -> Option<&mut Heading> {
        Some(self)
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

#[derive(Debug, Eq, PartialEq)]
pub struct Contents {
    pub title: Text,
    pub max_level: usize,
}

impl Contents {
    pub fn new() -> Contents {
        Default::default()
    }
}

impl BlockType for Contents {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref().map(|n| n.as_ref()) {
            Some("max_level") => {
                self.max_level = param
                    .1
                    .parse::<usize>()
                    .with_context(|_| ErrorKind::Parse)?;
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

#[derive(Debug, Default, Eq, PartialEq)]
pub struct List {
    pub items: Vec<ListItem>,
    pub ordered: bool,
}

impl List {
    pub fn new() -> List {
        Default::default()
    }
}

impl BlockType for List {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
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

    #[cfg(test)]
    fn as_list(&self) -> Option<&List> {
        Some(self)
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct ListItem {
    pub text: Text,
    pub sublist: Vec<ListItem>,
}

impl ListItem {
    pub fn new() -> ListItem {
        Default::default()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Table {
    pub title: Text,
    pub numbered: bool,
    pub rows: Vec<Row>,
    pub columns: Vec<Column>,
}

impl Table {
    pub fn new() -> Table {
        Default::default()
    }
}

impl BlockType for Table {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
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

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Row {
    pub cells: Vec<Cell>,
    pub header: bool,
    pub class: String,
}

impl Row {
    pub fn new() -> Row {
        Default::default()
    }
}

impl UpdateParam for Row {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref().map(|n| n.as_ref()) {
            Some("class") => {
                self.class = param.1;
                None
            }
            None => {
                match param.1.as_ref() {
                    "header" => self.header = true,
                    _ => self.class = param.1,
                }
                None
            }
            Some(_) => Some(param),
        })
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Column {
    pub header: bool,
    pub class: String,
}

impl Column {
    pub fn new() -> Column {
        Default::default()
    }
}

impl UpdateParam for Column {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref().map(|n| n.as_ref()) {
            Some("class") => {
                self.class = param.1;
                None
            }
            None => {
                match param.1.as_ref() {
                    "header" => self.header = true,
                    _ => self.class = param.1,
                }
                None
            }
            Some(_) => Some(param),
        })
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Cell {
    pub rows: usize,
    pub cols: usize,
    pub class: String,
    pub text: Text,
}

impl Cell {
    pub fn new() -> Cell {
        Default::default()
    }
}

impl UpdateParam for Cell {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref().map(|n| n.as_ref()) {
            Some("class") | None => {
                self.class = param.1;
                None
            }
            Some("rows") => {
                self.rows = param
                    .1
                    .parse::<usize>()
                    .with_context(|_| ErrorKind::Parse)?;
                None
            }
            Some("cols") => {
                self.cols = param
                    .1
                    .parse::<usize>()
                    .with_context(|_| ErrorKind::Parse)?;
                None
            }
            Some(_) => Some(param),
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Gloss {
    pub title: Text,
    pub numbered: bool,
    pub preamble: Vec<Text>,
    pub gloss: Vec<GlossLine>,
    pub postamble: Vec<Text>,
}

impl Gloss {
    pub fn new() -> Gloss {
        Default::default()
    }
}

impl BlockType for Gloss {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
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

#[derive(Debug, Default, Eq, PartialEq)]
pub struct GlossLine {
    pub words: Vec<Text>,
    pub class: String,
}

impl GlossLine {
    pub fn new() -> GlossLine {
        Default::default()
    }

    pub fn push(&mut self, word: Text) {
        self.words.push(word);
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum GlossLineType {
    NoSplit,
    Split,
}

impl GlossLineType {
    /// Updates with the given parameter. If the parameter was not updated, returns the parameter.
    pub fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref() {
            Some(_) => Some(param),
            None => match param.1.as_ref() {
                "nosplit" => {
                    *self = GlossLineType::NoSplit;
                    None
                }
                _ => Some(param),
            },
        })
    }
}
impl Default for GlossLineType {
    fn default() -> GlossLineType {
        GlossLineType::Split
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Text(pub Vec<Inline>);

impl Text {
    pub fn new() -> Text {
        Default::default()
    }

    pub fn push(&mut self, element: impl Into<Inline>) {
        self.0.push(element.into());
    }

    pub fn with_class(self, class: impl Into<String>) -> Text {
        Text(vec![Inline {
            kind: InlineType::Span(self),
            common: InlineCommon {
                class: class.into(),
            },
        }])
    }
}

impl BlockType for Text {}

impl<T> From<T> for Text
where
    T: Into<String>,
{
    fn from(s: T) -> Text {
        let mut t = Text::new();
        t.push(s.into());
        t
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Inline {
    pub kind: InlineType,
    pub common: InlineCommon,
}

impl<T> From<(InlineType, T)> for Inline
where
    T: Into<InlineCommon>,
{
    fn from((kind, common): (InlineType, T)) -> Inline {
        Inline {
            kind,
            common: common.into(),
        }
    }
}

impl From<String> for Inline {
    fn from(s: String) -> Inline {
        Inline::from((InlineType::Text(s), String::new()))
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct InlineCommon {
    pub class: String,
}

impl InlineCommon {
    pub fn new() -> InlineCommon {
        Default::default()
    }
}

impl UpdateParam for InlineCommon {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref().map(|n| n.as_ref()) {
            Some("class") | None => {
                self.class = param.1;
                None
            }
            _ => Some(param),
        })
    }
}

impl<T> From<T> for InlineCommon
where
    T: Into<String>,
{
    fn from(class: T) -> InlineCommon {
        InlineCommon {
            class: class.into(),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
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

impl InlineType {
    pub fn link() -> InlineType {
        InlineType::Link(Default::default())
    }

    pub fn reference() -> InlineType {
        InlineType::Reference(Default::default())
    }
}

impl UpdateParam for InlineType {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match *self {
            InlineType::Reference(ref mut s) => match param.0.as_ref().map(|p| p.as_ref()) {
                Some("ref") | None => {
                    *s = param.1;
                    None
                }
                _ => Some(param),
            },
            InlineType::Link(ref mut link) => match param.0.as_ref().map(|p| p.as_ref()) {
                Some("link") | None => {
                    link.url = param.1;
                    None
                }
                Some("title") => {
                    link.title = param.1.into();
                    None
                }
                _ => Some(param),
            },
            _ => Some(param),
        })
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Link {
    pub url: String,
    pub title: Text,
}
