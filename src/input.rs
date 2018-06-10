use std::io::{BufRead, Lines};
use std::iter::Enumerate;
use std::ops::Deref;

use failure::{err_msg, ResultExt};
use itertools::Itertools;

use document;
use errors::{ErrorKind, Result as EResult};

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
        match self.next() {
            Some(':') => match &*self.directive()? {
                "toc" => unimplemented!(),
                "list" => unimplemented!(),
                "table" => unimplemented!(),
                "gloss" => unimplemented!(),
                _ => unimplemented!(),
            },
            Some('#') => unimplemented!(),
            Some(_) => unimplemented!(),
            None => unimplemented!(),
        }
    }

    /// Returns a directive as a string, assuming the first `:` has already been parsed.
    fn directive(&mut self) -> EResult<String> {
        let start = self.index();
        while let Some(c) = self.next() {
            match c {
                ':' => {
                    let end = self.index();
                    return Ok(self[start..end].iter().collect());
                }
                '\\' => {
                    self.next();
                }
                '\n' => {
                    return self.error("Unexpected end of line while scanning for directive");
                }
                _ => {}
            }
        }
        self.error("Unexpected end of block while scanning for directive")
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
                    match self.peek() {
                        // end of the parameter list
                        Some(']') => {
                            self.idx += 1;
                            return Ok(params);
                        }
                        // something else: it's a parameter
                        Some(_) => {
                            self.parameter()?.map(|p| params.push(p));
                        }
                        // end of input!
                        None => {
                            return self.error(
                                "Unexpected end of block while scanning for parameters",
                            );
                        }
                    }
                }
            }
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
                    param_builder.last_mut().unwrap().push(self.expect()?);
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
            Ok(Some(Parameter(name, value)))
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
                    param_builder.last_mut().unwrap().push(self.expect()?);
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

    /// Pushes contents of a `{}`-delimited text to the given buffer, assuming the first `{` has
    /// already been matched.
    fn bracketed(&mut self, buffer: &mut String) -> EResult<()> {
        while let Some(c) = self.next() {
            match c {
                // done
                '}' => return Ok(()),
                // get the next character, whatever it may be.
                '\\' => buffer.push(self.expect()?),
                // otherwise, just push whatever we see.
                _ => buffer.push(c),
            }
        }
        self.error("Unexpected end of block while scanning for `}`")
    }

    /// Returns the next character, or an error if the end of the block is reached.
    fn expect(&mut self) -> EResult<char> {
        match self.next() {
            Some(c) => Ok(c),
            None => self.error("Unexpected end of block."),
        }
    }

    /// Returns an error with the given message, wrapped in an `ErrorKind::Block` and a `Result`.
    fn error<T>(&self, msg: &'static str) -> EResult<T> {
        Err(err_msg(msg)
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

struct Parameter(String, Option<String>);

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
