use std::fmt::Debug;
use std::io::{Result as IoResult, Write};
use std::ops::Deref;

use crate::blocks::{BlockCommon, BlockType, Parameter};
use crate::document::Document;
use crate::errors::Result as EResult;
use crate::html;
use crate::text::{Referenceable, Text, EMPTY_TEXT};

type OResult<T> = EResult<Option<T>>;

/// Writes a section number recursively.
fn write_section_number(w: &mut dyn Write, number: &[usize]) -> IoResult<()> {
    if let Some((last, rest)) = number.split_last() {
        write!(w, "<span class=\"secnum\">")?;
        write_section_number(w, rest)?;
        write!(w, "{}.</span>", last)?;
    }
    Ok(())
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
    fn write(&self, w: &mut dyn Write, common: &BlockCommon, document: &Document) -> IoResult<()> {
        // start tag
        write!(w, "<{} ", self.tag())?;
        write!(w, "id=\"{}\" ", html::Encoder(&common.id))?;
        write!(w, "class=\"{} ", html::Encoder(&common.class))?;
        if self.level > 6 {
            // we're just using a `p` tag, so the heading level must be specified as a class
            write!(w, " h{}\">", self.level)?;
        } else {
            // we're using a proper heading tag, so no need to specify the heading level as a class
            write!(w, "\">")?;
        }
        if self.numbered {
            write_section_number(w, &self.number)?;
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
pub struct FillerHeading {
    children: SectionList,
}

impl FillerHeading {
    pub fn new(level: usize) -> FillerHeading {
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
pub struct SectionList {
    pub headings: Vec<usize>,
    pub last_child_number: usize,
    pub level: usize,
}

impl SectionList {
    pub fn new(level: usize) -> SectionList {
        SectionList {
            level,
            ..Default::default()
        }
    }

    pub fn push(&mut self, index: usize, numbered: bool) {
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
