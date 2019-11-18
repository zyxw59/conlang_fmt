use std::io::{Result as IoResult, Write};

use crate::blocks::{BlockCommon, BlockType, Parameter, UpdateParam};
use crate::document::Document;
use crate::errors::Result as EResult;
use crate::html;

type OResult<T> = EResult<Option<T>>;

pub trait Referenceable {
    /// Outputs the text of a reference to the block.
    fn reference_text(&self) -> Text;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Text(pub Vec<Inline>);

pub const EMPTY_TEXT: &'static Text = &Text(Vec::new());

impl Text {
    pub fn new() -> Text {
        Default::default()
    }

    pub fn push(&mut self, element: impl Into<Inline>) {
        self.0.push(element.into());
    }

    pub fn extend(&mut self, other: &Text) {
        self.0.extend_from_slice(&other.0)
    }

    pub fn with_class(self, class: impl Into<String>) -> Text {
        Text(vec![Inline {
            kind: InlineType::Span(self),
            common: InlineCommon {
                class: class.into(),
            },
        }])
    }

    pub fn write_inline(&self, w: &mut dyn Write, document: &Document) -> IoResult<()> {
        for t in &self.0 {
            t.kind.write(w, &t.common, document)?;
        }
        Ok(())
    }

    /// Writes the text without any formatting (but still expanding replacements)
    pub fn write_inline_plain(&self, w: &mut dyn Write, document: &Document) -> IoResult<()> {
        for t in &self.0 {
            t.kind.write_plain(w, document)?;
        }
        Ok(())
    }

    pub fn starts_with(&self, c: char) -> bool {
        match self.0.first() {
            Some(inline) => inline.kind.starts_with(c),
            None => false,
        }
    }

    pub fn ends_with(&self, c: char) -> bool {
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

#[derive(Clone, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
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

    fn write(&self, w: &mut dyn Write, common: &InlineCommon, document: &Document) -> IoResult<()> {
        if let Some(tag) = self.tag() {
            write!(w, "<{} ", tag)?;
            write!(
                w,
                "class=\"{} {}\"",
                html::Encoder(self.class()),
                html::Encoder(&common.class)
            )?;
            if let InlineType::Link(link) = self {
                write!(w, " href=\"{}\"", html::Encoder(&link.url))?;
            } else if let InlineType::Reference(id) = self {
                write!(w, " href=\"#{}\"", html::Encoder(id))?;
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
            InlineType::Text(s) => write!(w, "{}", html::Encoder(s))?,
            InlineType::Reference(id) => {
                if let Some(block) = document.get_id(id) {
                    if let Some(referenceable) = block.kind.as_referenceable() {
                        referenceable.reference_text().write_inline(w, document)?;
                    } else {
                        write!(
                            w,
                            "<span class=\"unreferenceable-block\">#{}</span>",
                            html::Encoder(id)
                        )?;
                    }
                } else {
                    write!(
                        w,
                        "<span class=\"undefined-reference\">#{}</span>",
                        html::Encoder(id)
                    )?;
                }
            }
            InlineType::Replace(key) => match document.get_replacement(key) {
                Some(t) => t.write_inline(w, &document)?,
                None => {
                    write!(
                        w,
                        "<span class=\"undefined-replace\">:{}:</span>",
                        html::Encoder(key)
                    )?;
                }
            },
        }
        if let Some(tag) = self.tag() {
            write!(w, "</{}>", tag)?;
        }
        Ok(())
    }

    fn write_plain(&self, w: &mut dyn Write, document: &Document) -> IoResult<()> {
        match self {
            InlineType::Emphasis(t)
            | InlineType::Strong(t)
            | InlineType::Italics(t)
            | InlineType::Bold(t)
            | InlineType::SmallCaps(t)
            | InlineType::Span(t)
            | InlineType::Link(Link { title: t, .. }) => t.write_inline_plain(w, &document)?,
            InlineType::Text(s) => write!(w, "{}", html::Encoder(s))?,
            InlineType::Reference(id) => {
                if let Some(block) = document.get_id(id) {
                    if let Some(referenceable) = block.kind.as_referenceable() {
                        referenceable.reference_text().write_inline_plain(w, document)?;
                    } else {
                        write!(w, "#{}", html::Encoder(id))?;
                    }
                } else {
                    write!(w, "#{}", html::Encoder(id))?;
                }
            }
            InlineType::Replace(key) => match document.get_replacement(key) {
                Some(t) => t.write_inline_plain(w, &document)?,
                None => write!(w, ":{}:", html::Encoder(key))?,
            },
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Link {
    pub url: String,
    pub title: Text,
}
