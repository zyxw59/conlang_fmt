use std::ops::Deref;

use failure::{Fail, ResultExt};
use itertools::Itertools;

use crate::blocks::{self, Parameter, UpdateParam};
use crate::errors::{EndOfBlockKind, ErrorKind, Result as EResult};
use crate::text;

type OResult<T> = EResult<Option<T>>;

/// A slice of characters representing a block
#[derive(Debug)]
pub struct Block<'a> {
    slice: &'a [char],
    start: Option<usize>,
    idx: usize,
}

/// Update each object `$x` in order with the parameters returned by `$self.parameters()?`.
///
/// Uses `$self` to raise appropriate errors.
///
/// Panics if no argument in `$x` handles all cases where the parameter name is `None`.
macro_rules! update_multiple {
    ( $self:ident, $( $x:expr ),* ) => {
        {
            for param in $self.parameters()? {
                // `update_one!` does the heavy lifting.
                update_one!($self, param, $( $x ),*);
            }
        }
    }
}

/// Updates each object `$first, $x..` in order with the parameter `$param`.
///
/// If the parameter is returned by `$first`, move on to the first `$x`. If it is returned by
/// `$last`, raise an error by calling `$self.parameter_error(param.0.unwrap())?`.
///
/// Panics if no argument handles all cases where the parameter name is `None`.
macro_rules! update_one {
    ( $self:ident, $param:expr, $first: expr, $( $x:expr ),* ) => {
        {
            if let Some(param) = $first.update_param($param)? {
                // if the parameter is returned, try the next argument.
                update_one!($self, param, $( $x ),*)
            }
            // otherwise, we're done.
        }
    };
    ( $self:ident, $param:expr, $last:expr ) => {
        {
            if let Some(param) = $last.update_param($param)? {
                // we can unwrap because `common` will always catch the `None` case
                // (and treat it as a class).
                $self.parameter_error(param.0.unwrap())?
            }
        }
    };
}

macro_rules! push_and_renew {
    ($buffer:ident : $constructor:expr, $collector:ident) => {
        if $buffer.len() != 0 {
            $collector.push($buffer);
            $buffer = $constructor;
        }
    };
}

impl<'a> Block<'a> {
    pub fn new(slice: &'a [char], start: Option<usize>) -> Block<'a> {
        Block {
            slice,
            start,
            idx: 0,
        }
    }

    /// Parses the block.
    pub fn parse(&mut self) -> OResult<blocks::Block> {
        // skip leading whitespace
        self.skip_whitespace();
        // save the position of the first non-whitespace character; if we need to rewind, this is
        // where we should go.
        let start = self.idx;
        Ok(Some(match self.next() {
            Some(':') => match self.directive()?.as_ref() {
                "toc" => self.parse_toc()?,
                "list" => self.parse_list()?,
                "table" => self.parse_table()?,
                "gloss" => self.parse_gloss()?,
                "replace" => self.parse_replace_block()?,
                // any other directive is an inline directive; rewind and parse the block as a
                // paragraph
                _ => self.parse_paragraph(start)?,
            },
            Some('#') => self.parse_heading(start)?,
            Some(_) => self.parse_paragraph(start)?,
            None => return Ok(None),
        }))
    }

    fn parse_toc(&mut self) -> EResult<blocks::Block> {
        let mut toc = blocks::contents::Contents::new();
        let mut common = blocks::BlockCommon::new(self.start.unwrap());
        update_multiple!(self, toc, common);
        self.text_rest(&mut toc.title)?;
        Ok(blocks::Block {
            kind: Box::new(toc),
            common,
        })
    }

    fn parse_list(&mut self) -> EResult<blocks::Block> {
        let mut list = blocks::list::List::new();
        let mut common = blocks::BlockCommon::new(self.start.unwrap());
        update_multiple!(self, list, common);
        while self.idx < self.len() {
            let indent = self.skip_whitespace_virtual() - self.idx;
            self.idx += indent + 2;
            let mut item = blocks::list::ListItem::new();
            self.text_until_hard_line(&mut item.text)?;
            self.list_tree(indent, &mut item.sublist)?;
            list.items.push(item);
        }
        Ok(blocks::Block {
            kind: Box::new(list),
            common,
        })
    }

    fn parse_table(&mut self) -> EResult<blocks::Block> {
        let mut table = blocks::table::Table::new();
        let mut common = blocks::BlockCommon::new(self.start.unwrap());
        update_multiple!(self, table, common);
        self.text_until_char(&mut table.title, '\n')?;
        // put the newline back on the stack, since it's needed for `match_hard_line`
        self.idx -= 1;
        // match column parameters
        while let Some(c) = self.next() {
            match c {
                // new cell
                '|' => {
                    let mut col = blocks::table::Column::new();
                    update_multiple!(self, col);
                    table.columns.push(col);
                }
                // end of column parameter row
                c if self.match_hard_line(c) => break,
                // skip
                c if c.is_whitespace() => {}
                // error
                c => {
                    return Err(ErrorKind::Expected('|', c)
                        .context(ErrorKind::Block(self.start.unwrap()))
                        .into());
                }
            }
        }
        // now we've matched a hard line; time to start constructing the rows of the
        // table
        while let Some(_) = self.peek() {
            self.skip_whitespace();
            // skip until after the double colon
            self.idx += 2;
            let mut row = blocks::table::Row::new();
            update_multiple!(self, row);
            // match the cells
            while let Some(c) = self.next() {
                match c {
                    // new cell
                    '|' => {
                        let mut cell = blocks::table::Cell::new();
                        update_multiple!(self, cell);
                        self.text_until(&mut cell.text, |slf, c| {
                            c == '|' || slf.match_hard_line(c)
                        })?;
                        // rewind to put the pipe or newline back
                        self.idx -= 1;
                        row.cells.push(cell);
                        match self.peek() {
                            Some('|') => {}
                            _ => break,
                        }
                    }
                    '\n' if self.match_hard_line('\n') => break,
                    c if c.is_whitespace() => {}
                    c => {
                        return Err(ErrorKind::Expected('|', c)
                            .context(ErrorKind::Block(self.start.unwrap()))
                            .into());
                    }
                }
            }
            // now push the row and loop
            if !row.cells.is_empty() {
                table.rows.push(row);
            }
        }
        Ok(blocks::Block {
            kind: Box::new(table),
            common,
        })
    }

    fn parse_gloss(&mut self) -> EResult<blocks::Block> {
        let mut gloss = blocks::gloss::Gloss::new();
        let mut common = blocks::BlockCommon::new(self.start.unwrap());
        update_multiple!(self, gloss, common);
        self.text_until_hard_line(&mut gloss.title)?;
        // now we've matched a hard line; time to start constructing the lines of the
        // gloss
        while let Some(_) = self.peek() {
            self.skip_whitespace();
            // skip until after the double colon
            self.idx += 2;
            let mut class = String::new();
            let mut kind = blocks::gloss::GlossLineType::Split;
            update_multiple!(self, kind, class);
            // check whether it's a nosplit:
            match kind {
                blocks::gloss::GlossLineType::NoSplit => {
                    let mut line = Default::default();
                    // add the rest of the line
                    self.text_until_hard_line(&mut line)?;
                    // add class if there was one in the parameters
                    if !class.is_empty() {
                        line = line.with_class(class);
                    }
                    // if we've matched split lines, this must be in the postamble,
                    // otherwise it's the preamble
                    if gloss.gloss.is_empty() {
                        gloss.preamble.push(line);
                    } else {
                        gloss.postamble.push(line);
                    }
                }
                blocks::gloss::GlossLineType::Split => {
                    // check if we've already entered the postamble; a gloss line here
                    // is an error
                    if !gloss.postamble.is_empty() {
                        return Err(ErrorKind::GlossLine
                            .context(ErrorKind::Block(self.start.unwrap()))
                            .into());
                    }
                    let mut line = blocks::gloss::GlossLine::new();
                    line.class = class;
                    while let Some(c) = self.next() {
                        match c {
                            // break if we're at a hard line break
                            '\n' if self.match_hard_line('\n') => break,
                            // otherwise, skip whitespace
                            c if c.is_whitespace() => {}
                            // non-whitespace; start a new word
                            _ => {
                                let mut word = Default::default();
                                // rewind, since we want to include the character we
                                // matched
                                self.idx -= 1;
                                self.text_until(&mut word, |_, c| c.is_whitespace())?;
                                // rewind, since `text_until` consumes the whitespace
                                self.idx -= 1;
                                line.push(word);
                            }
                        }
                    }
                    gloss.gloss.push(line);
                }
            }
        }
        Ok(blocks::Block {
            kind: Box::new(gloss),
            common,
        })
    }

    fn parse_replace_block(&mut self) -> EResult<blocks::Block> {
        let mut replacements = blocks::replacements::Replacements::new();
        let mut common = blocks::BlockCommon::new(self.start.unwrap());
        update_multiple!(self, common);
        self.skip_whitespace();
        while let Some(':') = self.next() {
            let directive = self.directive()?;
            let mut text = text::Text::new();
            self.text_until_char(&mut text, '\n')?;
            replacements
                .insert(directive, text)
                .context(ErrorKind::Block(self.start.unwrap()))?;
        }
        Ok(blocks::Block {
            kind: Box::new(replacements),
            common,
        })
    }

    fn parse_heading(&mut self, start: usize) -> EResult<blocks::Block> {
        // count the `#`s
        while let Some('#') = self.next() {}
        // this is the number of `#`s. Subtract 1 because we're now at the char *after* the
        // last `#`.
        let level = self.idx - start - 1;
        // then rewind one character, we don't want to eat the character _after_ the `#`s.
        self.idx -= 1;
        let mut heading = blocks::heading::Heading::new(level);
        let mut common = blocks::BlockCommon::new(self.start.unwrap());
        update_multiple!(self, heading, common);
        self.text_rest(&mut heading.title)?;
        Ok(blocks::Block {
            kind: Box::new(heading),
            common,
        })
    }

    fn parse_paragraph(&mut self, start: usize) -> EResult<blocks::Block> {
        self.idx = start;
        let mut text = text::Text::new();
        let common = blocks::BlockCommon::new(self.start.unwrap());
        self.text_rest(&mut text)?;
        Ok(blocks::Block {
            kind: Box::new(text),
            common,
        })
    }

    /// Recursively appends list items to the given vector
    fn list_tree(
        &mut self,
        last_indent: usize,
        parent: &mut Vec<blocks::list::ListItem>,
    ) -> EResult<()> {
        loop {
            let indent = self.skip_whitespace_virtual() - self.idx;
            if indent <= last_indent {
                return Ok(());
            }
            self.idx += indent + 2;
            let mut item = blocks::list::ListItem::new();
            self.text_until_hard_line(&mut item.text)?;
            self.list_tree(indent, &mut item.sublist)?;
            parent.push(item);
        }
    }

    /// Returns a directive as a string, assuming the first `:` has already been parsed.
    fn directive(&mut self) -> EResult<String> {
        let mut directive = String::new();
        loop {
            match self.expect(':')? {
                ':' => return Ok(directive),
                '\\' => directive.push(self.expect_escaped()?),
                c => directive.push(c),
            }
        }
    }

    /// Returns a list of parameters. If a parameter list isn't present, returns an empty list and
    /// doesn't advance the iterator.
    fn parameters(&mut self) -> EResult<Vec<Parameter>> {
        // save the current position, so that if we fail to find a parameter list, we don't advance
        // the iterator.
        let idx = self.idx;
        self.skip_whitespace();
        let mut params = Vec::new();
        match self.peek() {
            Some('[') => {
                // skip the `[` we just matched
                self.idx += 1;
                // skip whitespace
                self.skip_whitespace();
                // loop over arguments
                loop {
                    match self.expect(']')? {
                        // end of the parameter list
                        ']' => return Ok(params),
                        // something else: it's a parameter
                        _ => {
                            // rewind, since the character we matched might be part of the
                            // parameter.
                            self.idx -= 1;
                            if let Some(p) = self.parameter()? {
                                params.push(p);
                            }
                        }
                    }
                }
            }
            // no parameter list, return an empty list and rewind the iterator
            _ => {
                self.idx = idx;
                Ok(params)
            }
        }
    }

    /// Matches a parameter.
    ///
    /// Leading and trailing whitespace is ignored, and all internal whitespace is replaced by a
    /// single space.
    fn parameter(&mut self) -> OResult<Parameter> {
        // skip leading whitespace
        self.skip_whitespace();
        // we'll build the parameter out of whitespace-separated strings, replacing all
        // intermediate whitespace with a single space.
        let mut param_builder = Vec::new();
        param_builder.push(String::new());
        let mut value = None;
        // loop over chars
        while let Some(c) = self.peek() {
            match c {
                // end of the parameter list; return what we have so far, but keep the `]` on the
                // stack.
                ']' => break,
                // get the next character, whatever it may be.
                '\\' => {
                    self.idx += 1;
                    param_builder
                        .last_mut()
                        .unwrap()
                        .push(self.expect_escaped()?);
                }
                // bracketed text
                '{' => {
                    self.idx += 1;
                    self.bracketed(&mut param_builder.last_mut().unwrap())?;
                }
                // end of this parameter; return what we have so far, and pop the `,`.
                ',' => {
                    self.idx += 1;
                    break;
                }
                // end of the parameter name; now get the value
                '=' => {
                    self.idx += 1;
                    value = Some(self.parameter_value()?);
                    break;
                }
                // skip whitespace, and start a new word.
                c if c.is_whitespace() => {
                    param_builder.push(String::new());
                    self.skip_whitespace();
                }
                // otherwise, push the char and keep going.
                c => {
                    self.idx += 1;
                    param_builder.last_mut().unwrap().push(c);
                }
            }
        }
        let name = param_builder.iter().filter(|w| !w.is_empty()).join(" ");
        if name.is_empty() {
            Ok(None)
        } else {
            match value {
                Some(value) => Ok(Some(Parameter(Some(name), value))),
                None => Ok(Some(Parameter(None, name))),
            }
        }
    }

    /// Matches a parameter value.
    ///
    /// Leading and trailing whitespace is ignored, and all internal whitespace is replaced by a
    /// single space.
    fn parameter_value(&mut self) -> EResult<String> {
        // skip leading whitespace
        self.skip_whitespace();
        // we'll build the parameter value out of whitespace-separated strings, replacing all
        // intermediate whitespace with a single space.
        let mut param_builder = Vec::new();
        param_builder.push(String::new());
        // loop over chars
        while let Some(c) = self.peek() {
            match c {
                // end of the parameter list; return what we have so far, but keep the `]` on the
                // stack.
                ']' => break,
                // get the next character, whatever it may be.
                '\\' => {
                    self.idx += 1;
                    param_builder
                        .last_mut()
                        .unwrap()
                        .push(self.expect_escaped()?);
                }
                // bracketed text
                '{' => {
                    self.idx += 1;
                    self.bracketed(&mut param_builder.last_mut().unwrap())?;
                }
                // end of this parameter; return what we have so far, and pop the `,`.
                ',' => {
                    self.idx += 1;
                    break;
                }
                // skip whitespace, and start a new word.
                c if c.is_whitespace() => {
                    param_builder.push(String::new());
                    self.skip_whitespace();
                }
                // otherwise, push the char and keep going.
                c => {
                    self.idx += 1;
                    param_builder.last_mut().unwrap().push(c);
                }
            }
        }
        Ok(param_builder.iter().filter(|w| !w.is_empty()).join(" "))
    }

    /// Pushes contents of a `{}`-delimited group to the given buffer, assuming the first `{` has
    /// already been matched.
    fn bracketed(&mut self, buffer: &mut String) -> EResult<()> {
        loop {
            match self.expect('}')? {
                // done
                '}' => return Ok(()),
                // get the next character, whatever it may be.
                '\\' => buffer.push(self.expect_escaped()?),
                // otherwise, just push whatever we see.
                c => buffer.push(c),
            }
        }
    }

    /// Appends elements to the given `text::Text` object up until the end of the block.
    fn text_rest(&mut self, text: &mut text::Text) -> EResult<()> {
        // never break
        self.text_until(text, |_, _| false)
    }

    /// Appends elements to the given `text::Text` object up until the next occurance of the
    /// specified `char` not contained in another element, or until the end of the block.
    fn text_until_char(&mut self, text: &mut text::Text, until: char) -> EResult<()> {
        self.text_until(text, |_, c| c == until)
    }

    /// Appends elements to the given `text::Text` object up until the next occurrance of `::`
    /// at the start of a line (ignoring whitespace), not contained in another element, or until
    /// the end of the block. The iterator will point at the first character of the line, which is
    /// either whitespace or the first colon.
    fn text_until_hard_line(&mut self, text: &mut text::Text) -> EResult<()> {
        self.text_until(text, Self::match_hard_line)
    }

    /// Matches a line starting with `::`.
    fn match_hard_line(&self, c: char) -> bool {
        let idx = self.skip_whitespace_virtual();
        // match the newline, and then...
        c == '\n'
            && match self.get(idx) {
                // match the first colon
                Some(':') => match self.get(idx + 1) {
                    // match the second colon: we're done
                    Some(':') => true,
                    _ => false,
                },
                // end of block after newline and whitespace; this is the end of a hard line
                None => true,
                _ => false,
            }
    }

    /// Appends elements to the given `text::Text` object up until the character matching the
    /// specified predicate not contained in another element, or until the end of the block.
    fn text_until(
        &mut self,
        text: &mut text::Text,
        predicate: impl Fn(&Self, char) -> bool,
    ) -> EResult<()> {
        let mut buffer = String::new();
        while let Some(c) = self.next() {
            match c {
                // the specified character was found, break
                c if predicate(self, c) => break,
                // bracketed text
                '{' => {
                    push_and_renew!(buffer: String::new(), text);
                    self.text_until_char(text, '}')?;
                }
                // directive
                ':' => {
                    push_and_renew!(buffer: String::new(), text);
                    text.push(match self.directive()?.as_ref() {
                        // cross reference
                        "ref" => self.simple_inline(text::InlineType::reference())?,
                        // link
                        "link" => self.simple_inline(text::InlineType::link())?,
                        // replacement
                        repl => self.simple_inline(text::InlineType::Replace(repl.into()))?,
                    });
                }
                // emphasis (semantic)
                '*' => {
                    push_and_renew!(buffer: String::new(), text);
                    text.push(self.formatting_inline(
                        '*',
                        text::InlineType::Emphasis,
                        text::InlineType::Strong,
                    )?);
                }
                // italics/bold (non-semantic)
                '_' => {
                    push_and_renew!(buffer: String::new(), text);
                    text.push(self.formatting_inline(
                        '_',
                        text::InlineType::Italics,
                        text::InlineType::Bold,
                    )?);
                }
                // small caps
                '^' => {
                    push_and_renew!(buffer: String::new(), text);
                    // rewind
                    let mut inner = text::Text::new();
                    self.text_until_char(&mut inner, '^')?;
                    let kind = text::InlineType::SmallCaps(inner);
                    text.push(self.simple_inline(kind)?);
                }
                // generic `span`
                '`' => {
                    push_and_renew!(buffer: String::new(), text);
                    let mut inner = text::Text::new();
                    self.text_until_char(&mut inner, '`')?;
                    let kind = text::InlineType::Span(inner);
                    let mut common = text::InlineCommon::new();
                    // defaults to a class of "conlang"
                    common.class = "conlang".into();
                    // we don't need to update `span`, because it has no parameters of its own
                    update_multiple!(self, common);
                    text.push(text::Inline { kind, common });
                }
                // escaped character
                '\\' => buffer.push(self.expect_escaped()?),
                // whitespace (only push one space, regardless of the amount or type of whitespace.
                c if c.is_whitespace() => {
                    self.skip_whitespace();
                    buffer.push(' ');
                }
                // anything else
                _ => buffer.push(c),
            }
        }
        if !buffer.is_empty() {
            text.push(buffer);
        }
        Ok(())
    }

    fn simple_inline(&mut self, mut kind: text::InlineType) -> EResult<text::Inline> {
        let mut common = text::InlineCommon::new();
        update_multiple!(self, kind, common);
        Ok(text::Inline { kind, common })
    }

    fn formatting_inline(
        &mut self,
        delim: char,
        single: impl FnOnce(text::Text) -> text::InlineType,
        double: impl FnOnce(text::Text) -> text::InlineType,
    ) -> EResult<text::Inline> {
        let kind = match self.expect(delim)? {
            // double
            c if c == delim => {
                let mut text = text::Text::new();
                self.text_until_char(&mut text, delim)?;
                self.expect_exact(delim)?;
                double(text)
            }
            // single
            _ => {
                // rewind
                self.idx -= 1;
                let mut text = text::Text::new();
                self.text_until_char(&mut text, delim)?;
                single(text)
            }
        };
        self.simple_inline(kind)
    }

    /// Returns the next character, or an error reporting which character is missing if the end of
    /// the block is reached.
    fn expect(&mut self, expected: char) -> EResult<char> {
        match self.next() {
            Some(c) => Ok(c),
            None => self.end_of_block(EndOfBlockKind::Expect(expected)),
        }
    }

    /// Returns the next character, or an error reporting that a character was expected if the end
    /// of the block is reached.
    fn expect_escaped(&mut self) -> EResult<char> {
        match self.next() {
            Some(c) => Ok(c),
            None => self.end_of_block(EndOfBlockKind::Escape),
        }
    }

    /// Returns an error if the next character is not the specified character, or if the end of the
    /// block is reached.
    fn expect_exact(&mut self, expected: char) -> EResult<()> {
        match self.next() {
            Some(c) if c == expected => Ok(()),
            Some(c) => Err(ErrorKind::Expected(expected, c)
                .context(ErrorKind::Block(self.start.unwrap()))
                .into()),
            None => self.end_of_block(EndOfBlockKind::Expect(expected)),
        }
    }

    /// Returns an `EndOfBlock` error, wrapped in a `Block` error and a `Result`
    fn end_of_block<T>(&self, kind: EndOfBlockKind) -> EResult<T> {
        Err(ErrorKind::EndOfBlock(kind)
            .context(ErrorKind::Block(self.start.unwrap()))
            .into())
    }

    /// Returns a `Parameter` error, wrapped in a `Block` error and a `Result`
    fn parameter_error<T>(&self, parameter: String) -> EResult<T> {
        Err(ErrorKind::Parameter(parameter)
            .context(ErrorKind::Block(self.start.unwrap()))
            .into())
    }

    /// Returns the starting line number of the block, which is only defined for non-empty blocks.
    pub fn start(&self) -> Option<usize> {
        self.start
    }

    /// Returns the next character in the block, advancing the iterator.
    fn next(&mut self) -> Option<char> {
        let c = self.slice.get(self.idx).cloned();
        self.idx += 1;
        c
    }

    /// Peeks at the next character in the block, without advancing the iterator.
    fn peek(&self) -> Option<char> {
        self.slice.get(self.idx).cloned()
    }

    /// Skips until the next non-whitespace character.
    fn skip_whitespace(&mut self) {
        self.idx = self.skip_whitespace_virtual();
    }

    /// Finds the index for the next non-whitespace character, or the end of the block, without
    /// advancing the iterator.
    fn skip_whitespace_virtual(&self) -> usize {
        let mut idx = self.idx;
        while let Some(c) = self.get(idx) {
            if !c.is_whitespace() {
                break;
            } else {
                idx += 1;
            }
        }
        idx
    }
}

impl<'a> Deref for Block<'a> {
    type Target = &'a [char];

    fn deref(&self) -> &&'a [char] {
        &self.slice
    }
}

#[cfg(test)]
mod tests {
    use crate::input::Input;

    use super::*;

    macro_rules! block {
        ($id:ident = $str:expr) => {
            let slice = $str.as_bytes();
            let mut input = Input::new(slice);
            let mut $id = input.next_block().unwrap();
        };
    }

    #[test]
    fn block_iter() {
        block!(block = r#"block 1, line 1"#);
        assert_eq!(block.start(), Some(0));
        assert_eq!(block.peek(), Some('b'));
        assert_eq!(block.next(), Some('b'));
    }

    macro_rules! parameter {
        ($value:tt) => {
            Parameter(None, $value.into())
        };
        ($name:tt: $value:tt) => {
            Parameter(Some($name.into()), $value.into())
        };
    }

    macro_rules! parameters {
        ($($name:tt $(: $value:tt)?),*) => {
            vec![$(parameter!($name $(: $value)?),)*]
        };
    }

    #[test]
    fn parameters_nameless() {
        block!(block = r#"[nameless]"#);
        assert_eq!(block.parameters().unwrap(), parameters!["nameless"]);
    }

    #[test]
    fn parameters_named() {
        block!(block = r#"[class=foo]"#);
        assert_eq!(block.parameters().unwrap(), parameters!["class": "foo"]);
    }

    #[test]
    fn parameters_space() {
        block!(block = r#"[ nameless ]"#);
        assert_eq!(block.parameters().unwrap(), parameters!["nameless"]);
    }

    #[test]
    fn parameters_multiple() {
        block!(block = r#"[id=foo, class=bar]"#);
        assert_eq!(
            block.parameters().unwrap(),
            parameters!["id": "foo", "class": "bar"]
        );
    }

    #[test]
    fn parameters_none() {
        block!(block = "0\n::");
        block.idx += 1;
        assert_eq!(block.parameters().unwrap(), parameters![]);
        assert_eq!(block.next().unwrap(), '\n');
        assert!(block.match_hard_line('\n'));
    }

    #[test]
    fn directive() {
        block!(block = ":foo:x");
        block.next();
        assert_eq!(block.directive().unwrap(), "foo");
        assert_eq!(block.next(), Some('x'));
    }

    macro_rules! text {
        ($($($type:ident)? ($text:tt)),*) => {
            $crate::text::Text(vec![$(inline!($($type)? ($text))),*])
        }
    }

    macro_rules! inline {
        ($type:ident ($text:tt)) => {
            $crate::text::Inline {
                kind: $crate::text::InlineType::$type($text.into()),
                common: Default::default(),
            }
        };
        (($text:tt)) => {
            String::from($text).into()
        };
    }

    #[test]
    fn text_emphasis() {
        block!(block = r#"*emphasis*"#);
        let mut text = text::Text::new();
        block.text_rest(&mut text).unwrap();
        assert_eq!(text, text!(Emphasis("emphasis"), (" ")))
    }

    #[test]
    fn text_strong() {
        block!(block = r#"**strong**"#);
        let mut text = text::Text::new();
        block.text_rest(&mut text).unwrap();
        assert_eq!(text, text!(Strong("strong"), (" ")))
    }

    macro_rules! list {
        ($($text:tt: [$($sl:tt)*]),*) => {
            vec![$(
                $crate::blocks::list::ListItem {
                    text: $text.into(),
                    sublist: list![$($sl)*],
                },
            )*]
        }
    }

    #[test]
    fn list() {
        block!(block = ":list:\n::1\n::2\n ::2a\n ::2b\n::3");
        let block = block.parse().unwrap().unwrap();
        let list = block.kind.as_list().unwrap();
        assert!(!list.ordered);
        assert_eq!(
            list.items,
            list!["1": [], "2": ["2a": [], "2b": []], "3": []]
        );
    }

    #[test]
    fn heading() {
        block!(block = "# Test");
        let block = block.parse().unwrap().unwrap();
        let got = block.kind.as_heading().unwrap();
        let expected = blocks::heading::Heading {
            title: " Test ".into(),
            numbered: true,
            toc: true,
            ..blocks::heading::Heading::new(1)
        };
        assert!(
            got.eq(&expected),
            r#"assertion failed: `(left == right)`
  left: `{:#?}`,
 right: `{:#?}`"#,
            &*got,
            &expected
        );
    }
}
