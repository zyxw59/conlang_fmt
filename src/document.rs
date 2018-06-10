use std::collections::HashMap;
use std::default::Default;

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

#[derive(Debug)]
pub struct Block {
    pub kind: BlockType,
    pub common: BlockCommon,
}

#[derive(Debug, Default)]
pub struct BlockCommon {
    pub class: String,
    pub id: String,
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

#[derive(Debug, Default)]
pub struct Heading {
    pub title: Text,
    pub numbered: bool,
    pub level: usize,
    pub children: Vec<usize>,
}

#[derive(Debug, Default)]
pub struct Contents {
    pub title: Text,
    pub max_level: usize,
}

#[derive(Debug, Default)]
pub struct List {
    pub items: Vec<ListItem>,
    pub ordered: bool,
}

#[derive(Debug, Default)]
pub struct ListItem {
    pub text: Text,
    pub sublist: Option<List>,
}

#[derive(Debug, Default)]
pub struct Table {
    pub title: Text,
    pub numbered: bool,
    pub rows: Vec<Row>,
    pub columns: Vec<Column>,
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

#[derive(Debug, Default)]
pub struct Gloss {
    pub title: Text,
    pub numbered: bool,
    pub preamble: Vec<Text>,
    pub gloss: Vec<Vec<Text>>,
    pub postamble: Vec<Text>,
}

pub type Text = Vec<Inline>;

#[derive(Debug)]
pub struct Inline {
    pub kind: InlineType,
    pub class: String,
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
}

#[derive(Debug, Default)]
pub struct Link {
    pub url: String,
    pub title: Text,
}
