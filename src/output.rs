use std::io::{self, BufWriter, Write};

use failure::{Context, Fail};
use htmlescape;

use errors::{ErrorKind, Result as EResult};

/// Unwraps the value contained in the enum variant specified, or panics if a different enum
/// variant is found.
macro_rules! unwrap_enum {
    ( $x:expr, $variant:path ) => {
        match $x {
            $variant(ref x) => x,
            _ => panic!("called `unwrap_enum!()` with a mismatching enum variant"),
        }
    }
}

trait OutputResult<T, E> {
    fn context(self) -> Result<T, Context<ErrorKind>>;
}

impl<T, E> OutputResult<T, E> for Result<T, E>
where
    E: Fail,
{
    fn context(self) -> Result<T, Context<ErrorKind>> {
        self.map_err(|err| err.context(ErrorKind::Output))
    }
}

/// Writes an attribute/value pair, escaping the value as necessary.
fn write_attribute(w: &mut impl Write, attr: &str, value: &str) -> Result<(), io::Error> {
    write!(w, r#"{}=""#, attr)?;
    htmlescape::encode_attribute_w(value, w)?;
    write!(w, r#"" "#)
}

/// Writes a section number recursively.
fn write_section_number(w: &mut impl Write, number: &[usize]) -> Result<(), io::Error> {
    write!(w, "<span ")?;
    write_attribute(w, "class", "secnum")?;
    write!(w, ">")?;
    if let Some((last, rest)) = number.split_last() {
        write_section_number(w, rest)?;
        write!(w, "{}.</span>", last)
    } else {
        write!(w, "</span>")
    }
}

struct Document {
    blocks: Vec<Block>,
    sections: Vec<usize>,
}

impl Document {
    fn output(&self, w: &mut BufWriter<impl Write>) -> EResult<()> {
        for block in self.blocks.iter() {
            block.output(w, &self)?;
        }
        Ok(())
    }
}

enum Block {
    Heading(Heading),
    Contents(Contents),
}

impl Block {
    fn output(&self, w: &mut BufWriter<impl Write>, document: &Document) -> EResult<()> {
        match self {
            Block::Heading(b) => b.output(w, document),
            Block::Contents(b) => b.output(w, document),
        }
    }
}

struct Heading {
    level: usize,
    id: String,
    class: String,
    numbered: bool,
    number: Vec<usize>,
    title: Text,
    children: Vec<usize>,
}

impl Heading {
    fn output(&self, w: &mut BufWriter<impl Write>, document: &Document) -> EResult<()> {
        // print start tag
        write!(w, "<{} ", self.tag()).context()?;
        // print id, which should be set by now
        write_attribute(w, "id", &self.id).context()?;
        // print classes, including a heading class if necessary
        write!(w, r#"class=""#).context()?;
        htmlescape::encode_attribute_w(&self.class, w).context()?;
        if let 1...6 = self.level {
            write!(w, r#"">"#).context()?;
        } else {
            write!(w, r#"h{}">"#, self.level).context()?;
        }
        // print section number (which should be set by now) if necessary
        if self.numbered {
            write_section_number(w, &self.number).context()?;
        }
        // print title
        self.title.output(w, document)?;
        // close up
        write!(w, "</ {}>", self.tag()).context()?;
        Ok(())
    }

    fn tag(&self) -> &'static str {
        match self.level {
            1 => "h1",
            2 => "h2",
            3 => "h3",
            4 => "h4",
            5 => "h5",
            6 => "h6",
            _ => "span",
        }
    }
}

struct Contents {}

impl Contents {
    fn output(&self, w: &mut BufWriter<impl Write>, document: &Document) -> EResult<()> {
        unimplemented!();
    }

    fn output_sublist(
        &self,
        w: &mut BufWriter<impl Write>,
        list: &[usize],
        document: &Document,
    ) -> EResult<()> {
        // only do something if the list is nonempty
        if !list.is_empty() {
            // print start tag
            write!(w, "<ol>").context()?;
            // flag for when we need to set list item number manually
            let mut manual_number = false;
            // loop over elements
            for &e in list {
                let heading = unwrap_enum!(document.blocks[e], Block::Heading);
                if !heading.numbered {
                    write!(w, r#"<li class="nonumber">"#).context()?;
                    manual_number = true;
                } else if manual_number {
                    write!(w, r#"<li value="{}">"#, heading.number.last().unwrap()).context()?;
                    manual_number = false;
                } else {
                    write!(w, "<li>").context()?;
                }
                heading.title.output(w, document)?;
                self.output_sublist(w, &heading.children, document)?;
                writeln!(w, "</li>").context()?;
            }
        }
        Ok(())
    }
}

struct Text;

impl Text {
    fn output(&self, w: &mut BufWriter<impl Write>, document: &Document) -> EResult<()> {
        unimplemented!();
    }
}
