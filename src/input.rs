use std::io::{BufRead, Lines};
use std::iter::Enumerate;
use std::ops::Deref;

use failure::{Fail, ResultExt};
use itertools::Itertools;

use document::{self, Parameter};
use errors::{EndOfBlockKind, ErrorKind, Result as EResult};

type OResult<T> = EResult<Option<T>>;

#[derive(Debug)]
pub struct Input<B> {
    lines: Enumerate<Lines<B>>,
    buffer: Vec<char>,
}

impl<B> Input<B>
where
    B: BufRead,
{
    pub fn new(input: B) -> Input<B> {
        Input {
            lines: input.lines().enumerate(),
            buffer: Vec::new(),
        }
    }

    /// Retrieves the next block from the input.
    ///
    /// Blocks are delimited by blank (all-whitespace) lines.
    ///
    /// An empty block signifies that the end of the input has been reached.
    pub fn next_block(&mut self) -> EResult<Block> {
        let mut start_line = None;
        // clear buffer
        self.buffer.clear();
        while let Some((line_number, line)) = self.lines.next() {
            // unwrap line
            let line = line.with_context(|e| ErrorKind::from_io(e, line_number))?;
            // blank lines
            if line.trim().len() == 0 {
                // if the buffer is empty, don't return anything
                if self.buffer.len() > 0 {
                    // but if it's not, we've reached the end of a block
                    break;
                }
            } else {
                if self.buffer.len() == 0 {
                    // if this is the first line of the block, set the start line
                    start_line = Some(line_number);
                }
                self.buffer.extend(line.chars());
                self.buffer.push('\n');
            }
        }
        // if we broke earlier, or if we've reached the end of the text, return the iterator.
        Ok(Block::new(self.buffer.as_ref(), start_line))
    }
}

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
        let mut block = match self.next() {
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
                    unimplemented!();
                }
                "table" => {
                    let mut table = document::Table::new();
                    let mut common = document::BlockCommon::new();
                    update_multiple!(self, table, common);
                    unimplemented!();
                }
                "gloss" => {
                    let mut gloss = document::Gloss::new();
                    let mut common = document::BlockCommon::new();
                    update_multiple!(self, gloss, common);
                    unimplemented!();
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
        };
        unimplemented!()
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

    /// Returns a list of parameters. If a parameter list isn't present, returns an empty list.
    fn parameters(&mut self) -> EResult<Vec<Parameter>> {
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
            // no parameter list, return an empty list
            _ => Ok(params),
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
        self.text_until(text, |_| true)
    }

    /// Appends elements to the given `document::Text` object up until the next occurance of the
    /// specified `char` not contained in another element, or until the end of the block.
    fn text_until_char(&mut self, text: &mut document::Text, until: char) -> EResult<()> {
        self.text_until(text, |c| c == until)
    }

    /// Appends elements to the given `document::Text` object up until the character matching the
    /// specified predicate not contained in another element, or until the end of the block.
    fn text_until(
        &mut self,
        text: &mut document::Text,
        predicate: impl Fn(char) -> bool,
    ) -> EResult<()> {
        let mut buffer = String::new();
        while let Some(c) = self.next() {
            match c {
                // the specified character was found, break
                c if predicate(c) => break,
                // bracketed text
                '{' => {
                    push_and_renew!(buffer: String::new(), text);
                    self.text_until_char(text, '}')?;
                }
                // directive
                ':' => {
                    push_and_renew!(buffer: String::new(), text);
                    match self.directive()?.as_ref() {
                        // cross reference
                        "ref" => {
                            let mut element = document::InlineType::reference();
                            let mut common = document::InlineCommon::new();
                            update_multiple!(self, element, common);
                            text.push((element, common));
                        }
                        // link
                        "link" => {
                            let mut element = document::InlineType::link();
                            let mut common = document::InlineCommon::new();
                            update_multiple!(self, element, common);
                            text.push((element, common));
                        }
                        // replacement
                        repl => {
                            let element = document::InlineType::Replace(repl.into());
                            let mut common = document::InlineCommon::new();
                            // we don't need to update `element`, because it has no parameters of
                            // its own
                            update_multiple!(self, common);
                            text.push((element, common));
                        }
                    }
                }
                // emphasis (semantic)
                '*' => {
                    push_and_renew!(buffer: String::new(), text);
                    let emph = match self.expect('*')? {
                        // strong emphasis
                        '*' => {
                            let mut inner = document::Text::new();
                            self.text_until_char(&mut inner, '*')?;
                            // if the next character is not '*', there was a stray single '*',
                            // which is an error.
                            self.expect_exact('*')?;
                            document::InlineType::Strong(inner)
                        }
                        // match the string.
                        _ => {
                            // rewind
                            self.idx -= 1;
                            let mut inner = document::Text::new();
                            self.text_until_char(&mut inner, '*')?;
                            document::InlineType::Emphasis(inner)
                        }
                    };
                    let mut common = document::InlineCommon::new();
                    // we don't need to update `emph`, because it has no parameters of its
                    // own
                    update_multiple!(self, common);
                    text.push((emph, common));
                }
                // italics/bold (non-semantic)
                '_' => {
                    push_and_renew!(buffer: String::new(), text);
                    let span = match self.expect('_')? {
                        // bold
                        '_' => {
                            let mut inner = document::Text::new();
                            self.text_until_char(&mut inner, '_')?;
                            // if the next character is not '_', there was a stray single '_',
                            // which is an error.
                            self.expect_exact('_')?;
                            document::InlineType::Bold(inner)
                        }
                        // match the string.
                        _ => {
                            // rewind
                            self.idx -= 1;
                            let mut inner = document::Text::new();
                            self.text_until_char(&mut inner, '_')?;
                            document::InlineType::Italics(inner)
                        }
                    };
                    let mut common = document::InlineCommon::new();
                    // we don't need to update `span`, because it has no parameters of its
                    // own
                    update_multiple!(self, common);
                    text.push((span, common));
                }
                // small caps
                '^' => {
                    push_and_renew!(buffer: String::new(), text);
                    // rewind
                    let mut inner = document::Text::new();
                    self.text_until_char(&mut inner, '^')?;
                    let span = document::InlineType::SmallCaps(inner);
                    let mut common = document::InlineCommon::new();
                    // we don't need to update `span`, because it has no parameters of its
                    // own
                    update_multiple!(self, common);
                    text.push((span, common));
                }
                // generic `span`
                '`' => {
                    push_and_renew!(buffer: String::new(), text);
                    let mut inner = document::Text::new();
                    self.text_until_char(&mut inner, '`')?;
                    let span = document::InlineType::Span(inner);
                    let mut common = document::InlineCommon::new();
                    // defaults to a class of "conlang"
                    common.class = "conlang".into();
                    // we don't need to update `span`, because it has no parameters of its own
                    update_multiple!(self, common);
                    text.push((span, common));
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

    /// Returns the length of the block, in number of characters.
    pub fn len(&self) -> usize {
        self.slice.len()
    }

    /// Returns the starting line number of the block, which is only defined for non-empty blocks.
    pub fn start(&self) -> Option<usize> {
        self.start
    }

    /// Returns the current index of the iterator.
    pub fn index(&self) -> usize {
        self.idx
    }

    /// Peeks at the next character in the block, without advancing the iterator.
    pub fn peek(&self) -> Option<char> {
        self.slice.get(self.idx + 1).cloned()
    }

    /// Skips until the next non-whitespace character.
    pub fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                return;
            } else {
                self.idx += 1;
            }
        }
    }
}

impl<'a> Iterator for Block<'a> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        self.idx += 1;
        self.slice.get(self.idx).cloned()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
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

    use super::*;

    #[test]
    fn blocks() {
        let input_str = r#"block 1, line 1
        block 1, line 2
        block 1, line 3

        block 2, line 1
        block 2, line 2
        "#.as_bytes();

        let mut input = Input::new(BufReader::new(input_str));

        {
            let block = input.next_block().unwrap();
            assert_eq!(block.start(), Some(0));
        }
        {
            let block = input.next_block().unwrap();
            assert_eq!(block.start(), Some(4));
        }
        {
            let block = input.next_block().unwrap();
            assert_eq!(block.len(), 0);
            assert_eq!(block.start(), None);
        }
    }

    #[test]
    fn extra_blank_lines() {
        let input_str = r#"block 1, line 1
        block 1, line 2
        block 1, line 3


        block 2, line 1
        block 2, line 2

        "#.as_bytes();

        let mut input = Input::new(BufReader::new(input_str));

        {
            let block = input.next_block().unwrap();
            assert_eq!(block.start(), Some(0));
        }
        {
            let block = input.next_block().unwrap();
            assert_eq!(block.start(), Some(5));
        }
        {
            let block = input.next_block().unwrap();
            assert_eq!(block.len(), 0);
            assert_eq!(block.start(), None);
        }
    }

    #[test]
    fn no_final_newline() {
        let input_str = r#"block 1, line 1
        block 1, line 2
        block 1, line 3"#.as_bytes();

        let mut input = Input::new(BufReader::new(input_str));

        {
            let block = input.next_block().unwrap();
            assert_eq!(block.start(), Some(0));
        }
        {
            let block = input.next_block().unwrap();
            assert_eq!(block.len(), 0);
            assert_eq!(block.start(), None);
        }
    }
}
