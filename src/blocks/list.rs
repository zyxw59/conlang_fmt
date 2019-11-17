use std::io::{Result as IoResult, Write};

use crate::blocks::{BlockCommon, BlockType, Parameter};
use crate::document::Document;
use crate::errors::Result as EResult;
use crate::html;
use crate::text::Text;

type OResult<T> = EResult<Option<T>>;

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
        w: &mut dyn Write,
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
    fn write(&self, w: &mut dyn Write, common: &BlockCommon, document: &Document) -> IoResult<()> {
        write!(w, "<{} ", List::tag(self.ordered))?;
        write!(w, "id=\"{}\" ", html::Encoder(&common.id))?;
        write!(w, "class=\"{}\">", html::Encoder(&common.class))?;
        List::write_list(w, &self.items, self.ordered, document)?;
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

    fn write(&self, w: &mut dyn Write, ordered: bool, document: &Document) -> IoResult<()> {
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
