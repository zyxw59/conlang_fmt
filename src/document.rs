use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::default::Default;
use std::fmt::Debug;
use std::io::{Result as IoResult, Write};

use failure::ResultExt;
use itertools::Itertools;

use crate::blocks::{
    control::DocumentControl,
    heading::{FillerHeading, HeadingLike, SectionList},
    replacements::Replacements,
    Block, BlockCommon,
};
use crate::errors::{ErrorKind, Result as EResult};
use crate::html;
use crate::text::Text;

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
    /// The title of the document.
    title: Option<String>,
    /// The author of the document.
    author: Option<String>,
    /// The description of the document.
    description: Option<String>,
    /// The stylesheets for the document.
    stylesheets: Vec<String>,
    /// The global `lang` attribute for the document.
    lang: Option<String>,
}

impl Document {
    /// Adds the given block to the document.
    pub fn add_block(&mut self, mut block: Block) -> EResult<()> {
        let mut idx = self.blocks.len();
        if let Some(control) = block.kind.as_control() {
            self.control(control);
        }
        if let Some(heading) = block.kind.as_mut_heading() {
            idx = self.add_heading(heading, &mut block.common)?;
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

    fn control(&mut self, control: &DocumentControl) {
        match control {
            DocumentControl::Title(s) => {
                self.title.get_or_insert(s.clone());
            }
            DocumentControl::Author(s) => {
                self.author.get_or_insert(s.clone());
            }
            DocumentControl::Description(s) => {
                self.description.get_or_insert(s.clone());
            }
            DocumentControl::Stylesheet(s) => {
                self.stylesheets.push(s.clone());
            }
            DocumentControl::Lang(s) => {
                self.lang.get_or_insert(s.clone());
            }
        }
    }

    fn add_heading(
        &mut self,
        heading: &mut dyn HeadingLike,
        common: &mut BlockCommon,
    ) -> EResult<usize> {
        let mut idx = self.blocks.len();
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
            if common.id.is_empty() {
                common.id = format!("sec-{}", heading.number().iter().format("-"));
            }
        }
        self.get_mut_section_list(curr)
            .push(idx, heading.numbered());
        Ok(idx)
    }

    /// Writes the blocks as HTML.
    pub fn write(&self, w: &mut impl Write) -> EResult<()> {
        self.write_head(w).context(ErrorKind::WriteIoHead)?;
        for Block { kind, common } in &self.blocks {
            kind.write(w, common, self)
                .context(ErrorKind::WriteIo(common.start_line))?;
        }
        self.write_tail(w).context(ErrorKind::WriteIoTail)?;
        Ok(())
    }

    fn write_head(&self, w: &mut impl Write) -> IoResult<()> {
        writeln!(w, "<!doctype html>")?;
        write!(w, "<html")?;
        if let Some(lang) = &self.lang {
            writeln!(w, " lang=\"{}\">", html::Encoder(lang))?;
        } else {
            writeln!(w, ">")?;
        }
        writeln!(w, "<head>")?;
        writeln!(w, "<meta charset=\"utf-8\" />")?;
        if let Some(title) = &self.title {
            writeln!(w, "<title>{}</title>", html::Encoder(title))?;
        }
        if let Some(author) = &self.author {
            writeln!(
                w,
                "<meta name=\"author\" content=\"{}\" />",
                html::Encoder(author)
            )?;
        }
        if let Some(description) = &self.description {
            writeln!(
                w,
                "<meta name=\"description\" content=\"{}\" />",
                html::Encoder(description)
            )?;
        }
        for stylesheet in &self.stylesheets {
            writeln!(
                w,
                "<link rel=\"stylesheet\" type=\"text/css\" href=\"{}\" />",
                html::Encoder(stylesheet)
            )?;
        }
        writeln!(w, "</head>")?;
        writeln!(w, "<body>")?;
        Ok(())
    }

    fn write_tail(&self, w: &mut impl Write) -> IoResult<()> {
        writeln!(w, "</body>")?;
        writeln!(w, "</html>")?;
        Ok(())
    }

    /// Get a reference to the specified block.
    pub fn get_block(&self, idx: usize) -> Option<&Block> {
        self.blocks.get(idx)
    }

    /// Get a reference to the specified block as a heading.
    ///
    /// Panics if the specified block doesn't exist or isn't a heading.
    pub fn get_heading(&self, block_index: usize) -> &dyn HeadingLike {
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
    pub fn get_section_list(&self, block_index: Option<usize>) -> &SectionList {
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
    pub fn get_id(&self, id: &str) -> Option<&Block> {
        self.ids.get(id).map(|&idx| &self.blocks[idx])
    }

    /// Gets the replacement text for the given key.
    pub fn get_replacement(&self, key: &str) -> Option<&Text> {
        self.replacements.get(key)
    }
}
