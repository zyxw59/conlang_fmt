use std::collections::HashMap;

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

pub struct Block {
    kind: BlockType,
    class: String,
    id: String,
}

pub enum BlockType {
    Heading(Heading),
    Contents(Contents),
    List(List),
    Table(Table),
    Gloss(Gloss),
    Paragraph(Text),
}

pub struct Heading {
    pub title: Text,
    pub numbered: bool,
    pub level: usize,
    pub children: Vec<usize>,
}

pub struct Contents {
    pub title: Text,
    pub max_level: usize,
}

pub struct List {
    pub items: Vec<ListItem>,
    pub ordered: bool,
}

pub struct ListItem {
    pub text: Text,
    pub sublist: Option<List>,
}

pub struct Table {
    pub title: Text,
    pub numbered: bool,
    pub rows: Vec<Row>,
    pub columns: Vec<Column>,
}

pub struct Row {
    pub cells: Vec<Cell>,
    pub header: bool,
    pub class: String,
}

pub struct Column {
    pub header: bool,
    pub class: String,
}

pub struct Cell {
    rows: usize,
    cols: usize,
    text: Text,
}

pub struct Gloss {
    pub title: Text,
    pub numbered: bool,
    pub preamble: Vec<Text>,
    pub gloss: Vec<Vec<Text>>,
    pub postamble: Vec<Text>,
}

pub type Text = Vec<Inline>;

pub struct Inline {
    pub kind: InlineType,
    pub class: String,
}

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

pub struct Link {
    pub url: String,
    pub title: Text,
}
