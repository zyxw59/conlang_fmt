use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::default::Default;
use std::fmt::Debug;
use std::io::{Result as IoResult, Write};
use std::ops::Deref;

use failure::ResultExt;
use htmlescape::encode_minimal_w;
use itertools::Itertools;

use crate::errors::{ErrorKind, Result as EResult};

type OResult<T> = EResult<Option<T>>;

/// Writes an attribute/value pair, escaping the value as necessary.
fn write_attribute(w: &mut impl Write, attr: &str, value: &str) -> IoResult<()> {
    write!(w, r#"{}=""#, attr)?;
    encode_minimal_w(value, w)?;
    write!(w, r#"" "#)
}

/// Writes a section number recursively.
fn write_section_number(w: &mut impl Write, number: &[usize]) -> IoResult<()> {
    if let Some((last, rest)) = number.split_last() {
        write!(w, "<span ")?;
        write_attribute(w, "class", "secnum")?;
        write!(w, ">")?;
        write_section_number(w, rest)?;
        write!(w, "{}.</span>", last)?;
    }
    Ok(())
}

#[derive(Debug, Default)]
pub struct Document {
    /// A list of blocks in the document
    blocks: Vec<Block>,
    /// A list of indices into the `blocks` field corresponding to the top-level section headings
    /// of the document.
    sections: SectionList,
    /// A map from IDs to indices into the `blocks` field.
    ids: HashMap<String, usize>,
    /// A map of defined replacements.
    replacements: Replacements,
    /// A list of indices into the `blocks` field corresponding to the tables.
    tables: Vec<usize>,
    /// A list of indices into the `blocks` field corresponding to the glosses.
    glosses: Vec<usize>,
    /// The last table number.
    table_number: usize,
    /// The last gloss number.
    gloss_number: usize,
    /// The first unused number for blocks without an ID.
    noid_index: usize,
}

impl Document {
    /// Adds the given block to the document.
    pub fn add_block(&mut self, mut block: Block) -> EResult<()> {
        let mut idx = self.blocks.len();
        if let Some(heading) = block.kind.as_mut_heading() {
            let mut curr = None;
            while self.get_section_list(curr).level < heading.level() {
                let curr_level = self.get_section_list(curr).level;
                if self.get_section_list(curr).is_empty() {
                    // insert filler section
                    self.blocks.push(FillerHeading::new(curr_level + 1).into());
                    self.get_mut_section_list(curr).push(idx, false);
                    // since we inserted another block before the one we're working on
                    idx += 1;
                }
                if heading.numbered() {
                    heading.push_number(self.get_section_list(curr).last_child_number);
                }
                // move to next child
                curr = self.get_section_list(curr).last().cloned();
            }
            // now, insert the heading into its direct parent.
            if !heading.numbered() {
                // if this is a nonumber heading, its last_child_number is the same as it's older
                // sibling's, if such a sibling exists (otherwise last_child_number should remain
                // the default 0)
                if let Some(&older_sibling) = self.get_section_list(curr).last() {
                    heading.mut_children().last_child_number =
                        self.get_section_list(Some(older_sibling)).last_child_number;
                }
            }
            if heading.numbered() {
                heading.push_number(self.get_section_list(curr).last_child_number + 1);
                if block.common.id.is_empty() {
                    block.common.id = format!("sec-{}", heading.number().iter().format("-"));
                }
            }
            self.get_mut_section_list(curr)
                .push(idx, heading.numbered());
        }
        if let Some(replacements) = block.kind.as_mut_replacements() {
            self.replacements.update(replacements);
        }
        if let Some(table) = block.kind.as_mut_table() {
            if table.numbered {
                self.table_number += 1;
                table.number = self.table_number;
            }
            self.tables.push(idx);
        }
        if let Some(gloss) = block.kind.as_mut_gloss() {
            if gloss.numbered {
                self.gloss_number += 1;
                gloss.number = self.gloss_number;
            }
            self.glosses.push(idx);
        }
        if block.common.id.is_empty() {
            block.common.id = format!("__no-id-{}", self.noid_index);
            self.noid_index += 1;
        }
        let id = block.common.id.clone();
        match self.ids.entry(id) {
            Entry::Occupied(e) => return Err(ErrorKind::Id(e.key().clone()).into()),
            Entry::Vacant(e) => e.insert(idx),
        };
        self.blocks.push(block);
        Ok(())
    }

    /// Writes the blocks as HTML.
    pub fn write(&self, w: &mut impl Write) -> EResult<()> {
        for Block { kind, common } in &self.blocks {
            kind.write(w, common, self)
                .context(ErrorKind::WriteIo(common.start_line))?;
        }
        Ok(())
    }

    /// Get a reference to the specified block.
    fn get_block(&self, idx: usize) -> Option<&Block> {
        self.blocks.get(idx)
    }

    /// Get a reference to the specified block as a heading.
    ///
    /// Panics if the specified block doesn't exist or isn't a heading.
    fn get_heading(&self, block_index: usize) -> &dyn HeadingLike {
        self.blocks[block_index].kind.as_heading().unwrap()
    }

    /// Get a mutable reference to the specified block as a heading.
    ///
    /// Panics if the specified block doesn't exist or isn't a heading.
    fn get_mut_heading(&mut self, block_index: usize) -> &mut dyn HeadingLike {
        self.blocks[block_index].kind.as_mut_heading().unwrap()
    }

    /// Get a reference to the children of the specified block, or the root section list if none is
    /// specified.
    ///
    /// Panics if the specified block doesn't exist or isn't a heading.
    fn get_section_list(&self, block_index: Option<usize>) -> &SectionList {
        if let Some(idx) = block_index {
            self.get_heading(idx).children()
        } else {
            &self.sections
        }
    }

    /// Get a mutable reference to the children of the specified block, or the root section list if
    /// none is specified.
    ///
    /// Panics if the specified block doesn't exist or isn't a heading.
    fn get_mut_section_list(&mut self, block_index: Option<usize>) -> &mut SectionList {
        if let Some(idx) = block_index {
            self.get_mut_heading(idx).mut_children()
        } else {
            &mut self.sections
        }
    }

    /// Gets a reference to the block with the specified ID.
    fn get_id(&self, id: &str) -> Option<&Block> {
        self.ids.get(id).map(|&idx| &self.blocks[idx])
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct SectionList {
    headings: Vec<usize>,
    last_child_number: usize,
    level: usize,
}

impl SectionList {
    fn new(level: usize) -> SectionList {
        SectionList {
            level,
            ..Default::default()
        }
    }

    fn push(&mut self, index: usize, numbered: bool) {
        self.headings.push(index);
        if numbered {
            self.last_child_number += 1;
        }
    }
}

impl Default for SectionList {
    fn default() -> SectionList {
        SectionList {
            headings: Default::default(),
            last_child_number: 0,
            level: 1,
        }
    }
}

impl Deref for SectionList {
    type Target = [usize];

    fn deref(&self) -> &[usize] {
        &self.headings
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

pub trait BlockType: Debug {
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
}

impl<T: BlockType> UpdateParam for T {
    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        BlockType::update_param(self, param)
    }
}

pub trait Referenceable {
    /// Outputs the text of a reference to the block.
    fn write_reference(&self, w: &mut dyn Write, document: &Document) -> IoResult<()>;
}

pub trait HeadingLike: Debug {
    fn numbered(&self) -> bool;
    fn toc(&self) -> bool;
    fn level(&self) -> usize;
    fn children(&self) -> &SectionList;
    fn mut_children(&mut self) -> &mut SectionList;
    fn number(&self) -> &[usize];
    fn push_number(&mut self, value: usize);
    fn title(&self) -> &Text;

    #[cfg(test)]
    fn eq(&self, other: &dyn HeadingLike) -> bool {
        self.numbered() == other.numbered()
            && self.toc() == other.toc()
            && self.level() == other.level()
            && self.children() == other.children()
            && self.number() == other.number()
            && self.title() == other.title()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Heading {
    pub title: Text,
    pub numbered: bool,
    pub toc: bool,
    pub level: usize,
    pub children: SectionList,
    pub number: Vec<usize>,
}

impl Heading {
    pub fn new(level: usize) -> Heading {
        Heading {
            level,
            children: SectionList::new(level + 1),
            ..Default::default()
        }
    }

    fn tag(&self) -> &'static str {
        match self.level {
            1 => "h1",
            2 => "h2",
            3 => "h3",
            4 => "h4",
            5 => "h5",
            6 => "h6",
            _ => "p",
        }
    }
}

impl BlockType for Heading {
    fn write(
        &self,
        mut w: &mut dyn Write,
        common: &BlockCommon,
        document: &Document,
    ) -> IoResult<()> {
        // start tag
        write!(w, "<{} ", self.tag())?;
        write_attribute(&mut w, "id", &common.id)?;
        write!(w, r#"class=""#)?;
        encode_minimal_w(&common.class, &mut w)?;
        if self.level > 6 {
            // we're just using a `p` tag, so the heading level must be specified as a class
            write!(w, r#" h{}">"#, self.level)?;
        } else {
            // we're using a proper heading tag, so no need to specify the heading level as a class
            write!(w, r#"">"#)?;
        }
        if self.numbered {
            write_section_number(&mut w, &self.number)?;
        }
        self.title.write_inline(w, &document)?;
        writeln!(w, "</{}>\n", self.tag())
    }

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

    fn as_referenceable(&self) -> Option<&dyn Referenceable> {
        Some(self)
    }

    fn as_heading(&self) -> Option<&dyn HeadingLike> {
        Some(self)
    }

    fn as_mut_heading(&mut self) -> Option<&mut dyn HeadingLike> {
        Some(self)
    }
}

impl Referenceable for Heading {
    fn write_reference(&self, mut w: &mut dyn Write, document: &Document) -> IoResult<()> {
        write!(w, "section ")?;
        if self.numbered {
            write_section_number(&mut w, &self.number)?;
        } else {
            self.title.write_inline(w, document)?;
        }
        Ok(())
    }
}

impl HeadingLike for Heading {
    fn numbered(&self) -> bool {
        self.numbered && self.toc
    }

    fn toc(&self) -> bool {
        self.toc
    }

    fn level(&self) -> usize {
        self.level
    }

    fn children(&self) -> &SectionList {
        &self.children
    }

    fn mut_children(&mut self) -> &mut SectionList {
        &mut self.children
    }

    fn number(&self) -> &[usize] {
        &self.number
    }

    fn push_number(&mut self, value: usize) {
        self.number.push(value);
    }

    fn title(&self) -> &Text {
        &self.title
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
            number: Default::default(),
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
struct FillerHeading {
    children: SectionList,
}

impl FillerHeading {
    fn new(level: usize) -> FillerHeading {
        FillerHeading {
            children: SectionList {
                level,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

impl BlockType for FillerHeading {
    fn write(&self, _: &mut dyn Write, _: &BlockCommon, _: &Document) -> IoResult<()> {
        Ok(())
    }

    fn as_heading(&self) -> Option<&dyn HeadingLike> {
        Some(self)
    }

    fn as_mut_heading(&mut self) -> Option<&mut dyn HeadingLike> {
        Some(self)
    }
}

impl HeadingLike for FillerHeading {
    fn numbered(&self) -> bool {
        false
    }

    fn toc(&self) -> bool {
        false
    }

    fn level(&self) -> usize {
        self.children.level - 1
    }

    fn children(&self) -> &SectionList {
        &self.children
    }

    fn mut_children(&mut self) -> &mut SectionList {
        &mut self.children
    }

    fn number(&self) -> &[usize] {
        &[]
    }

    fn push_number(&mut self, _: usize) {}

    fn title(&self) -> &Text {
        EMPTY_TEXT
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

    fn write_sublist(
        &self,
        w: &mut impl Write,
        level: usize,
        list: &[usize],
        document: &Document,
    ) -> IoResult<()> {
        if !list.is_empty() && level <= self.max_level {
            writeln!(w, "<ol>")?;
            // flag for when we need to set number manually.
            let mut manual_number = false;
            if let Some(&e) = list.first() {
                if let Some(&number) = document.get_heading(e).number().last() {
                    manual_number = number != 1;
                }
            }
            for &e in list {
                let heading = document.get_heading(e);
                if !heading.numbered() {
                    write!(w, r#"<li class="nonumber">"#)?;
                    manual_number = true;
                } else if manual_number {
                    write!(w, r#"<li value="{}">"#, heading.number().last().unwrap())?;
                    manual_number = false;
                } else {
                    write!(w, "<li>")?;
                }
                if heading.toc() {
                    write!(w, "<a href=\"#")?;
                    encode_minimal_w(&document.get_block(e).unwrap().common.id, w)?;
                    write!(w, "\">")?;
                    heading.title().write_inline(w, document)?;
                    write!(w, "</a>")?;
                }
                self.write_sublist(w, level + 1, heading.children(), &document)?;
                writeln!(w, "</li>")?;
            }
            writeln!(w, "</ol>\n")?;
        }
        Ok(())
    }
}

impl BlockType for Contents {
    fn write(
        &self,
        mut w: &mut dyn Write,
        common: &BlockCommon,
        document: &Document,
    ) -> IoResult<()> {
        write!(w, "<div ")?;
        write_attribute(&mut w, "id", &common.id)?;
        write!(w, r#"class=""#)?;
        encode_minimal_w(&common.class, &mut w)?;
        write!(w, " toc")?;
        write!(w, r#"">"#)?;
        write!(w, "<p ")?;
        write_attribute(&mut w, "class", "toc-heading")?;
        write!(w, ">")?;
        self.title.write_inline(w, &document)?;
        writeln!(w, "</p>")?;
        self.write_sublist(&mut w, 1, &document.sections, &document)?;
        writeln!(w, "</div>\n")
    }

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

    fn tag(ordered: bool) -> &'static str {
        if ordered {
            "ol"
        } else {
            "ul"
        }
    }

    fn write_list(
        w: &mut impl Write,
        items: &[ListItem],
        ordered: bool,
        document: &Document,
    ) -> IoResult<()> {
        for item in items {
            item.write(w, ordered, document)?;
        }
        Ok(())
    }
}

impl BlockType for List {
    fn write(
        &self,
        mut w: &mut dyn Write,
        common: &BlockCommon,
        document: &Document,
    ) -> IoResult<()> {
        write!(w, "<{} ", List::tag(self.ordered))?;
        write_attribute(&mut w, "id", &common.id)?;
        write_attribute(&mut w, "class", &common.class)?;
        writeln!(w, ">")?;
        List::write_list(&mut w, &self.items, self.ordered, document)?;
        write!(w, "</{}>\n", List::tag(self.ordered))
    }

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

    fn write(&self, w: &mut impl Write, ordered: bool, document: &Document) -> IoResult<()> {
        write!(w, "<li>")?;
        self.text.write_inline(w, document)?;
        if !self.sublist.is_empty() {
            writeln!(w, "<{}>", List::tag(ordered))?;
            List::write_list(w, &self.sublist, ordered, document)?;
            writeln!(w, "</{}>", List::tag(ordered))?;
        }
        writeln!(w, "</li>")
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Table {
    pub title: Text,
    pub numbered: bool,
    pub number: usize,
    pub rows: Vec<Row>,
    pub columns: Vec<Column>,
}

impl Table {
    pub fn new() -> Table {
        Default::default()
    }
}

impl BlockType for Table {
    fn write(
        &self,
        mut w: &mut dyn Write,
        common: &BlockCommon,
        document: &Document,
    ) -> IoResult<()> {
        write!(w, "<table ")?;
        write_attribute(&mut w, "id", &common.id)?;
        write_attribute(&mut w, "class", &common.class)?;
        writeln!(w, ">")?;
        write!(w, "<caption>")?;
        write!(w, r#"<span class="table-heading-prefix">Table"#)?;
        if self.numbered {
            write!(w, " {}", self.number)?;
        }
        write!(w, ":</span> ")?;
        self.title.write_inline(w, document)?;
        writeln!(w, "</caption>")?;
        // for recording when a cell is a continuation from an earlier row, to correctly count
        // columns
        let mut continuation_cells = Vec::<usize>::with_capacity(self.columns.len());
        for row in &self.rows {
            write!(w, "<tr ")?;
            write_attribute(&mut w, "class", &row.class)?;
            write!(w, ">")?;
            let mut col = 0;
            for cell in &row.cells {
                // increment col until we get to a free column
                while let Some(n) = continuation_cells.get_mut(col) {
                    if *n > 0 {
                        // decrement n while we're at it.
                        *n -= 1;
                        col += 1;
                    } else {
                        break;
                    }
                }
                // update continuation_cells if this cell has rowspan or colspan greater than 1
                // first, resize `continuation_cells` so that it can hold all the columns.
                if continuation_cells.len() < col + cell.cols {
                    continuation_cells.resize(col + cell.cols, 0);
                }
                for n in &mut continuation_cells[col..col + cell.cols] {
                    *n = cell.rows.max(*n).saturating_sub(1);
                }
                cell.write(&mut w, row, self.columns.get(col), document)?;
                col += cell.cols;
            }
            writeln!(w, "</tr>")?;
        }
        writeln!(w, "</table>\n")
    }

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

    fn as_mut_table(&mut self) -> Option<&mut Table> {
        Some(self)
    }

    fn as_referenceable(&self) -> Option<&dyn Referenceable> {
        Some(self)
    }
}

impl Referenceable for Table {
    fn write_reference(&self, w: &mut dyn Write, document: &Document) -> IoResult<()> {
        if self.numbered {
            write!(w, "table {}", self.number)?;
        } else {
            write!(w, "table ")?;
            self.title.write_inline(w, document)?;
        }
        Ok(())
    }
}

impl Default for Table {
    fn default() -> Table {
        Table {
            title: Default::default(),
            numbered: true,
            number: 0,
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

#[derive(Debug, Eq, PartialEq)]
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

    fn write(
        &self,
        w: &mut impl Write,
        row: &Row,
        col: Option<&Column>,
        document: &Document,
    ) -> IoResult<()> {
        let header_row = row.header;
        let header_col = col.map(|col| col.header).unwrap_or(false);
        if header_row {
            write!(w, "<th ")?;
            if self.cols > 1 {
                write_attribute(w, "scope", "colgroup")?;
            } else {
                write_attribute(w, "scope", "col")?;
            }
        } else if header_col {
            write!(w, "<th ")?;
            if self.rows > 1 {
                write_attribute(w, "scope", "rowgroup")?;
            } else {
                write_attribute(w, "scope", "row")?;
            }
        } else {
            write!(w, "<td ")?;
        }
        if self.cols > 1 {
            write_attribute(w, "colspan", &format!("{}", self.cols))?;
        }
        if self.rows > 1 {
            write_attribute(w, "rowspan", &format!("{}", self.rows))?;
        }
        write!(w, r#"class=""#)?;
        encode_minimal_w(&self.class, w)?;
        if let Some(col) = col {
            write!(w, " ")?;
            encode_minimal_w(&col.class, w)?;
        }
        write!(w, r#"">"#)?;
        self.text.write_inline(w, document)?;
        if header_row || header_col {
            write!(w, "</th>")?;
        } else {
            write!(w, "</td>")?;
        }
        Ok(())
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

impl Default for Cell {
    fn default() -> Cell {
        Cell {
            rows: 1,
            cols: 1,
            class: Default::default(),
            text: Default::default(),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Gloss {
    pub title: Text,
    pub numbered: bool,
    pub number: usize,
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
    fn write(
        &self,
        mut w: &mut dyn Write,
        common: &BlockCommon,
        document: &Document,
    ) -> IoResult<()> {
        write!(w, "<div ")?;
        write_attribute(&mut w, "id", &common.id)?;
        write!(w, r#"class="gloss "#)?;
        encode_minimal_w(&common.class, &mut w)?;
        write!(w, r#"">"#)?;
        write!(w, r#"<p class="gloss-heading">"#)?;
        write!(w, r#"<span class="gloss-heading-prefix">Gloss"#)?;
        if self.numbered {
            write!(w, " {}", self.number)?;
        }
        write!(w, ":</span> ")?;
        self.title.write_inline(w, document)?;
        writeln!(w, "</p>")?;
        for line in &self.preamble {
            write!(w, r#"<p class="preamble">"#)?;
            line.write_inline(w, document)?;
            writeln!(w, "</p>")?;
        }
        // get the length of the longest gloss line. If there are no lines, skip writing the gloss
        if let Some(num_words) = self.gloss.iter().map(|line| line.words.len()).max() {
            // flag whether to add a space before the next word.
            let mut add_space = false;
            for i in 0..num_words {
                let head_word = self.gloss[0].words.get(i);
                let is_prefix = match head_word {
                    Some(word) => word.starts_with('-'),
                    None => false,
                };
                if add_space || !is_prefix {
                    write!(w, " ")?;
                }
                write!(w, "<dl>")?;
                write!(w, "<dt ")?;
                write_attribute(&mut w, "class", &self.gloss[0].class)?;
                write!(w, ">")?;
                if let Some(text) = head_word {
                    text.write_inline(w, document)?;
                }
                write!(w, "</dt>")?;
                for line in &self.gloss[1..] {
                    write!(w, "<dd ")?;
                    write_attribute(&mut w, "class", &line.class)?;
                    write!(w, ">")?;
                    if let Some(text) = line.words.get(i) {
                        text.write_inline(w, document)?;
                    }
                    write!(w, "</dd>")?;
                }
                write!(w, "</dl>")?;
                add_space = match head_word {
                    Some(word) => word.ends_with('-'),
                    None => false,
                };
            }
        }
        for line in &self.postamble {
            write!(w, r#"<p class="postamble">"#)?;
            line.write_inline(w, document)?;
            writeln!(w, "</p>")?;
        }
        writeln!(w, "</div>\n")?;
        Ok(())
    }

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

    fn as_mut_gloss(&mut self) -> Option<&mut Gloss> {
        Some(self)
    }

    fn as_referenceable(&self) -> Option<&dyn Referenceable> {
        Some(self)
    }
}

impl Referenceable for Gloss {
    fn write_reference(&self, w: &mut dyn Write, document: &Document) -> IoResult<()> {
        if self.numbered {
            write!(w, "gloss {}", self.number)?;
        } else {
            write!(w, "gloss ")?;
            self.title.write_inline(w, document)?;
        }
        Ok(())
    }
}

impl Default for Gloss {
    fn default() -> Gloss {
        Gloss {
            title: Default::default(),
            numbered: true,
            number: 0,
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
pub struct Replacements {
    pub replacements: HashMap<String, Text>,
}

impl Replacements {
    pub fn new() -> Replacements {
        Default::default()
    }

    /// Inserts the given key/value pair, returning an error if the key is already present.
    pub fn insert(&mut self, key: String, value: Text) -> EResult<()> {
        if self.replacements.contains_key(&key) {
            Err(ErrorKind::Replace(key).into())
        } else {
            self.replacements.insert(key, value);
            Ok(())
        }
    }

    /// Updates `self` with keys from `other`, replacing duplicates.
    fn update(&mut self, other: &mut Replacements) {
        for (k, v) in other.drain() {
            self.replacements.insert(k, v);
        }
    }

    fn drain(&mut self) -> impl Iterator<Item = (String, Text)> + '_ {
        self.replacements.drain()
    }

    /// Gets the given key.
    fn get(&self, key: &str) -> Option<&Text> {
        self.replacements.get(key)
    }
}

impl BlockType for Replacements {
    fn write(&self, _: &mut dyn Write, _: &BlockCommon, _: &Document) -> IoResult<()> {
        Ok(())
    }

    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(Some(param))
    }

    fn as_mut_replacements(&mut self) -> Option<&mut Replacements> {
        Some(self)
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Text(pub Vec<Inline>);

const EMPTY_TEXT: &'static Text = &Text(Vec::new());

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

    fn write_inline(&self, w: &mut dyn Write, document: &Document) -> IoResult<()> {
        for t in &self.0 {
            t.kind.write(w, &t.common, document)?;
        }
        Ok(())
    }

    fn starts_with(&self, c: char) -> bool {
        match self.0.first() {
            Some(inline) => inline.kind.starts_with(c),
            None => false,
        }
    }

    fn ends_with(&self, c: char) -> bool {
        match self.0.last() {
            Some(inline) => inline.kind.ends_with(c),
            None => false,
        }
    }
}

impl BlockType for Text {
    fn write(&self, w: &mut dyn Write, _common: &BlockCommon, document: &Document) -> IoResult<()> {
        write!(w, "<p>")?;
        self.write_inline(w, document)?;
        writeln!(w, "</p>\n")?;
        Ok(())
    }
}

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

impl Inline {}

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

    fn write(
        &self,
        mut w: &mut dyn Write,
        common: &InlineCommon,
        document: &Document,
    ) -> IoResult<()> {
        if let Some(tag) = self.tag() {
            write!(w, "<{} ", tag)?;
            write!(w, r#"class="{} "#, self.class())?;
            encode_minimal_w(&common.class, &mut w)?;
            write!(w, r#"" "#)?;
            if let InlineType::Link(link) = self {
                write_attribute(&mut w, "href", &link.url)?;
            } else if let InlineType::Reference(id) = self {
                write!(w, "href=\"#")?;
                encode_minimal_w(id, &mut w)?;
                write!(w, "\"")?;
            }
            write!(w, ">")?;
        }
        match self {
            InlineType::Emphasis(t)
            | InlineType::Strong(t)
            | InlineType::Italics(t)
            | InlineType::Bold(t)
            | InlineType::SmallCaps(t)
            | InlineType::Span(t)
            | InlineType::Link(Link { title: t, .. }) => t.write_inline(w, &document)?,
            InlineType::Text(s) => write!(w, "{}", s)?,
            InlineType::Reference(id) => {
                if let Some(block) = document.get_id(id) {
                    if let Some(referenceable) = block.kind.as_referenceable() {
                        referenceable.write_reference(w, document)?;
                    } else {
                        write!(w, "<span class=\"unreferenceable-block\">#")?;
                        encode_minimal_w(id, &mut w)?;
                        write!(w, "</span>")?;
                    }
                } else {
                    write!(w, "<span class=\"undefined-reference\">#")?;
                    encode_minimal_w(id, &mut w)?;
                    write!(w, "</span>")?;
                }
            }
            InlineType::Replace(id) => match document.replacements.get(id) {
                Some(t) => t.write_inline(w, &document)?,
                None => {
                    write!(w, r#"<span class="undefined-replace">:"#)?;
                    encode_minimal_w(id, &mut w)?;
                    write!(w, ":</span>")?;
                }
            },
        }
        if let Some(tag) = self.tag() {
            write!(w, "</{}>", tag)?;
        }
        Ok(())
    }

    fn tag(&self) -> Option<&'static str> {
        use self::InlineType::*;
        match self {
            Emphasis(_) => Some("em"),
            Strong(_) => Some("strong"),
            Italics(_) => Some("i"),
            Bold(_) => Some("b"),
            Link(_) | Reference(_) => Some("a"),
            Text(_) => None,
            _ => Some("span"),
        }
    }

    fn class(&self) -> &'static str {
        use self::InlineType::*;
        match self {
            SmallCaps(_) => "small-caps",
            Reference(_) => "reference",
            _ => "",
        }
    }

    fn starts_with(&self, c: char) -> bool {
        match self {
            InlineType::Emphasis(t)
            | InlineType::Strong(t)
            | InlineType::Italics(t)
            | InlineType::Bold(t)
            | InlineType::SmallCaps(t)
            | InlineType::Span(t)
            | InlineType::Link(Link { title: t, .. }) => t.starts_with(c),
            InlineType::Text(s) => s.starts_with(c),
            _ => false,
        }
    }

    fn ends_with(&self, c: char) -> bool {
        match self {
            InlineType::Emphasis(t)
            | InlineType::Strong(t)
            | InlineType::Italics(t)
            | InlineType::Bold(t)
            | InlineType::SmallCaps(t)
            | InlineType::Span(t)
            | InlineType::Link(Link { title: t, .. }) => t.ends_with(c),
            InlineType::Text(s) => s.ends_with(c),
            _ => false,
        }
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
