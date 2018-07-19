use std::ops::Deref;

use failure::Fail;
use itertools::Itertools;

use document::{self, Parameter, UpdateParam};
use errors::{EndOfBlockKind, ErrorKind, Result as EResult};

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
            match $first.update_param($param)? {
                // if the parameter is returned, try the next argument.
                Some(param) => update_one!($self, param, $( $x ),*),
                // otherwise, we're done.
                None => {}
            }
        }
    };
    ( $self:ident, $param:expr, $last:expr ) => {
        {
            match $last.update_param($param)? {
                // we can unwrap because `common` will always catch the `None` case
                // (and treat it as a class).
                Some(param) => $self.parameter_error(param.0.unwrap())?,
                None => {}
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
    pub fn parse(&mut self) -> OResult<document::Block> {
        // skip leading whitespace
        self.skip_whitespace();
        // save the position of the first non-whitespace character; if we need to rewind, this is
        // where we should go.
        let start = self.idx;
        Ok(Some(match self.next() {
            Some(':') => match self.directive()?.as_ref() {
                "toc" => {
                    let mut toc = document::Contents::new();
                    let mut common = document::BlockCommon::new();
                    update_multiple!(self, toc, common);
                    self.text_rest(&mut toc.title)?;
                    document::Block {
                        kind: document::BlockType::Contents(toc),
                        common,
                    }
                }
                "list" => {
                    let mut list = document::List::new();
                    let mut common = document::BlockCommon::new();
                    update_multiple!(self, list, common);
                    while self.idx < self.len() {
                        let indent = self.skip_whitespace_virtual() - self.idx;
                        self.idx += indent + 2;
                        let mut item = document::ListItem::new();
                        self.text_until_hard_line(&mut item.text)?;
                        self.list_tree(indent, &mut item.sublist)?;
                        list.items.push(item);
                    }
                    document::Block {
                        kind: document::BlockType::List(list),
                        common,
                    }
                }
                "table" => {
                    let mut table = document::Table::new();
                    let mut common = document::BlockCommon::new();
                    update_multiple!(self, table, common);
                    self.text_until_char(&mut table.title, '\n')?;
                    // match column parameters
                    while let Some(c) = self.next() {
                        match c {
                            // new cell
                            '|' => {
                                let mut col = document::Column::new();
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
                                    .into())
                            }
                        }
                    }
                    // now we've matched a hard line; time to start constructing the rows of the
                    // table
                    while let Some(_) = self.peek() {
                        self.skip_whitespace();
                        // skip until after the double colon
                        self.idx += 2;
                        let mut row = document::Row::new();
                        update_multiple!(self, row);
                        // match the cells
                        while let Some(c) = self.next() {
                            match c {
                                // new cell
                                '|' => {
                                    let mut cell = document::Cell::new();
                                    update_multiple!(self, cell);
                                    self.text_until(&mut cell.text, |slf, c| {
                                        c == '|' || slf.match_hard_line(c)
                                    })?;
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
                                        .into())
                                }
                            }
                        }
                        // now push the row and loop
                        table.rows.push(row);
                    }
                    document::Block {
                        kind: document::BlockType::Table(table),
                        common,
                    }
                }
                "gloss" => {
                    let mut gloss = document::Gloss::new();
                    let mut common = document::BlockCommon::new();
                    update_multiple!(self, gloss, common);
                    self.text_until_hard_line(&mut gloss.title)?;
                    // now we've matched a hard line; time to start constructing the lines of the
                    // gloss
                    while let Some(_) = self.peek() {
                        self.skip_whitespace();
                        // skip until after the double colon
                        self.idx += 2;
                        let mut class = String::new();
                        let mut kind = document::GlossLineType::Split;
                        update_multiple!(self, kind, class);
                        // check whether it's a nosplit:
                        match kind {
                            document::GlossLineType::NoSplit => {
                                let mut line = Default::default();
                                // add the rest of the line
                                self.text_until_hard_line(&mut line)?;
                                // add class if there was one in the parameters
                                if class.len() != 0 {
                                    line = line.with_class(class);
                                }
                                // if we've matched split lines, this must be in the postamble,
                                // otherwise it's the preamble
                                if gloss.gloss.len() == 0 {
                                    gloss.preamble.push(line);
                                } else {
                                    gloss.postamble.push(line);
                                }
                            }
                            document::GlossLineType::Split => {
                                // check if we've already entered the postamble; a gloss line here
                                // is an error
                                if gloss.postamble.len() != 0 {
                                    return Err(ErrorKind::GlossLine
                                        .context(ErrorKind::Block(self.start.unwrap()))
                                        .into());
                                }
                                let mut line = document::GlossLine::new();
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
                    document::Block {
                        kind: document::BlockType::Gloss(gloss),
                        common,
                    }
                }
                _ => {
                    // this directive is an inline directive; rewind and parse the block as a
                    // paragraph
                    self.idx = start;
                    let mut text = document::Text::new();
                    self.text_rest(&mut text)?;
                    text.into()
                }
            },
            Some('#') => {
                // count the `#`s
                while let Some('#') = self.next() {}
                // this is the number of `#`s.
                let level = self.idx - start;
                // then rewind one character, we don't want to eat the character _after_ the `#`s.
                self.idx -= 1;
                let mut heading = document::Heading::new();
                heading.level = level;
                let mut common = document::BlockCommon::new();
                update_multiple!(self, heading, common);
                self.text_rest(&mut heading.title)?;
                document::Block {
                    kind: document::BlockType::Heading(heading),
                    common,
                }
            }
            Some(_) => {
                self.idx = start;
                let mut text = document::Text::new();
                self.text_rest(&mut text)?;
                text.into()
            }
            None => return Ok(None),
        }))
    }

    /// Recursively appends list items to the given vector
    fn list_tree(
        &mut self,
        last_indent: usize,
        parent: &mut Vec<document::ListItem>,
    ) -> EResult<()> {
        loop {
            let indent = self.skip_whitespace_virtual() - self.idx;
            if indent <= last_indent {
                return Ok(());
            }
            self.idx += indent + 2;
            let mut item = document::ListItem::new();
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
                            self.parameter()?.map(|p| params.push(p));
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
        let name = param_builder.iter().filter(|w| w.len() > 0).join(" ");
        if name.len() == 0 {
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
        Ok(param_builder.iter().filter(|w| w.len() > 0).join(" "))
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

    /// Appends elements to the given `document::Text` object up until the end of the block.
    fn text_rest(&mut self, text: &mut document::Text) -> EResult<()> {
        // never break
        self.text_until(text, |_, _| false)
    }

    /// Appends elements to the given `document::Text` object up until the next occurance of the
    /// specified `char` not contained in another element, or until the end of the block.
    fn text_until_char(&mut self, text: &mut document::Text, until: char) -> EResult<()> {
        self.text_until(text, |_, c| c == until)
    }

    /// Appends elements to the given `document::Text` object up until the next occurrance of `::`
    /// at the start of a line (ignoring whitespace), not contained in another element, or until
    /// the end of the block. The iterator will point at the first character of the line, which is
    /// either whitespace or the first colon.
    fn text_until_hard_line(&mut self, text: &mut document::Text) -> EResult<()> {
        self.text_until(text, Self::match_hard_line)
    }

    /// Matches a line starting with `::`.
    fn match_hard_line(&self, c: char) -> bool {
        let idx = self.skip_whitespace_virtual();
        // match the newline, and then...
        c == '\n' && match self.get(idx) {
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

    /// Appends elements to the given `document::Text` object up until the character matching the
    /// specified predicate not contained in another element, or until the end of the block.
    fn text_until(
        &mut self,
        text: &mut document::Text,
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
                        "ref" => self.simple_inline(document::InlineType::reference())?,
                        // link
                        "link" => self.simple_inline(document::InlineType::link())?,
                        // replacement
                        repl => self.simple_inline(document::InlineType::Replace(repl.into()))?,
                    });
                }
                // emphasis (semantic)
                '*' => {
                    push_and_renew!(buffer: String::new(), text);
                    text.push(self.formatting_inline(
                        '*',
                        document::InlineType::Emphasis,
                        document::InlineType::Strong,
                    )?);
                }
                // italics/bold (non-semantic)
                '_' => {
                    push_and_renew!(buffer: String::new(), text);
                    text.push(self.formatting_inline(
                        '*',
                        document::InlineType::Italics,
                        document::InlineType::Bold,
                    )?);
                }
                // small caps
                '^' => {
                    push_and_renew!(buffer: String::new(), text);
                    // rewind
                    let mut inner = document::Text::new();
                    self.text_until_char(&mut inner, '^')?;
                    let kind = document::InlineType::SmallCaps(inner);
                    text.push(self.simple_inline(kind)?);
                }
                // generic `span`
                '`' => {
                    push_and_renew!(buffer: String::new(), text);
                    let mut inner = document::Text::new();
                    self.text_until_char(&mut inner, '`')?;
                    let kind = document::InlineType::Span(inner);
                    let mut common = document::InlineCommon::new();
                    // defaults to a class of "conlang"
                    common.class = "conlang".into();
                    // we don't need to update `span`, because it has no parameters of its own
                    update_multiple!(self, common);
                    text.push(document::Inline { kind, common });
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
        if buffer.len() != 0 {
            text.push(buffer);
        }
        Ok(())
    }

    fn simple_inline(&mut self, mut kind: document::InlineType) -> EResult<document::Inline> {
        let mut common = document::InlineCommon::new();
        update_multiple!(self, kind, common);
        Ok(document::Inline { kind, common })
    }

    fn formatting_inline(
        &mut self,
        delim: char,
        single: impl FnOnce(document::Text) -> document::InlineType,
        double: impl FnOnce(document::Text) -> document::InlineType,
    ) -> EResult<document::Inline> {
        let kind = match self.expect(delim)? {
            // double
            c if c == delim => {
                let mut text = document::Text::new();
                self.text_until_char(&mut text, delim)?;
                self.expect_exact(delim)?;
                double(text)
            }
            // single
            _ => {
                // rewind
                self.idx -= 1;
                let mut text = document::Text::new();
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
    use std::io::BufReader;

    use input::Input;

    use super::*;

    #[test]
    fn block_iter() {
        let input_str = r#"block 1, line 1"#.as_bytes();
        let mut input = Input::new(BufReader::new(input_str));
        let mut block = input.next_block().unwrap();
        assert_eq!(block.start(), Some(0));
        assert_eq!(block.peek(), Some('b'));
        assert_eq!(block.next(), Some('b'));
    }

    #[test]
    fn parameters_nameless() {
        let input_str = r#"[nameless]"#.as_bytes();
        let mut input = Input::new(BufReader::new(input_str));
        let mut block = input.next_block().unwrap();
        let param = block.parameters().unwrap();
        assert_eq!(param, vec![Parameter(None, String::from("nameless"))]);
    }

    #[test]
    fn parameters_named() {
        let input_str = r#"[class=foo]"#.as_bytes();
        let mut input = Input::new(BufReader::new(input_str));
        let mut block = input.next_block().unwrap();
        let param = block.parameters().unwrap();
        assert_eq!(
            param,
            vec![Parameter(Some(String::from("class")), String::from("foo"))]
        );
    }

    #[test]
    fn parameters_space() {
        let input_str = r#"[ nameless ]"#.as_bytes();
        let mut input = Input::new(BufReader::new(input_str));
        let mut block = input.next_block().unwrap();
        let param = block.parameters().unwrap();
        assert_eq!(param, vec![Parameter(None, String::from("nameless"))]);
    }

    #[test]
    fn parameters_multiple() {
        let input_str = r#"[id=foo, class=bar]"#.as_bytes();
        let mut input = Input::new(BufReader::new(input_str));
        let mut block = input.next_block().unwrap();
        let param = block.parameters().unwrap();
        assert_eq!(
            param,
            vec![
                Parameter(Some(String::from("id")), String::from("foo")),
                Parameter(Some(String::from("class")), String::from("bar")),
            ]
        );
    }

    #[test]
    fn parameters_none() {
        let input_str = "0\n::".as_bytes();
        let mut input = Input::new(BufReader::new(input_str));
        let mut block = input.next_block().unwrap();
        block.idx += 1;
        let param = block.parameters().unwrap();
        assert_eq!(param, Vec::new());
        let newline = block.next().unwrap();
        assert_eq!(newline, '\n');
        assert!(block.match_hard_line(newline));
    }

    #[test]
    fn directive() {
        let input_str = ":foo:x".as_bytes();
        let mut input = Input::new(BufReader::new(input_str));
        let mut block = input.next_block().unwrap();
        block.next();
        let dir = block.directive().unwrap();
        assert_eq!(dir, "foo");
        assert_eq!(block.next(), Some('x'));
    }

    #[test]
    fn text_emphasis() {
        let input_str = r#"*emphasis*"#.as_bytes();
        let mut input = Input::new(BufReader::new(input_str));
        let mut block = input.next_block().unwrap();
        let mut text = document::Text::new();
        assert!(block.text_rest(&mut text).is_ok());
        assert_eq!(
            text,
            document::Text(vec![
                document::Inline {
                    kind: document::InlineType::Emphasis("emphasis".into()),
                    common: Default::default(),
                },
                document::Inline::from(String::from(" ")),
            ])
        )
    }

    #[test]
    fn text_strong() {
        let input_str = r#"**strong**"#.as_bytes();
        let mut input = Input::new(BufReader::new(input_str));
        let mut block = input.next_block().unwrap();
        let mut text = document::Text::new();
        assert!(block.text_rest(&mut text).is_ok());
        assert_eq!(
            text,
            document::Text(vec![
                document::Inline {
                    kind: document::InlineType::Strong("strong".into()),
                    common: Default::default(),
                },
                document::Inline::from(String::from(" ")),
            ])
        )
    }

    #[test]
    fn list() {
        let input_str = ":list:\n::1\n::2\n ::2a\n ::2b\n::3".as_bytes();
        let mut input = Input::new(BufReader::new(input_str));
        let block = input.next_block().unwrap().parse().unwrap().unwrap();
        if let document::BlockType::List(list) = block.kind {
            assert!(!list.ordered);
            assert_eq!(
                list.items[0],
                document::ListItem {
                    text: "1".into(),
                    sublist: vec![],
                }
            );
            assert_eq!(
                list.items[1],
                document::ListItem {
                    text: "2".into(),
                    sublist: vec![
                        document::ListItem {
                            text: "2a".into(),
                            sublist: vec![],
                        },
                        document::ListItem {
                            text: "2b".into(),
                            sublist: vec![],
                        },
                    ],
                }
            );
            assert_eq!(
                list.items[2],
                document::ListItem {
                    text: "3".into(),
                    sublist: vec![],
                }
            );
        } else {
            unreachable!();
        }
    }
}
