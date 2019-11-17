use std::io::{Result as IoResult, Write};

use htmlescape::encode_minimal_w;

use crate::blocks::{BlockCommon, BlockType, Parameter};
use crate::document::{write_attribute, Document};
use crate::errors::Result as EResult;
use crate::text::{Referenceable, Text};

type OResult<T> = EResult<Option<T>>;

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
