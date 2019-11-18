use std::io::{Result as IoResult, Write};

use failure::ResultExt;

use crate::blocks::{BlockCommon, BlockType, Parameter};
use crate::document::Document;
use crate::errors::{ErrorKind, Result as EResult};
use crate::html;
use crate::text::Text;

type OResult<T> = EResult<Option<T>>;

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
        w: &mut dyn Write,
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
                    write!(
                        w,
                        "<a href=\"#{}\">",
                        &document.get_block(e).unwrap().common.id
                    )?;
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
    fn write(&self, w: &mut dyn Write, common: &BlockCommon, document: &Document) -> IoResult<()> {
        write!(w, "<div ")?;
        write!(w, "id=\"{}\" ", html::Encoder(&common.id))?;
        write!(w, "class=\"{} toc\">", html::Encoder(&common.class))?;
        write!(w, "<p class=\"toc-heading\">")?;
        self.title.write_inline(w, &document)?;
        writeln!(w, "</p>")?;
        self.write_sublist(w, 1, document.get_section_list(None), &document)?;
        writeln!(w, "</div>\n")
    }

    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(match param.0.as_ref().map(|n| n.as_ref()) {
            Some("maxlevel") => {
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
