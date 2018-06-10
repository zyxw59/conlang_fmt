use std::io::{BufRead, Lines};
use std::iter::Enumerate;
use std::ops::Deref;
use std::vec::Drain;

use failure::ResultExt;

use errors::{Error, ErrorKind};
use document;

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
    pub fn next_block(&mut self) -> Result<Block, Error> {
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
    pub fn parse(&mut self) -> Result<Option<document::Block>, Error> {
        // skip leading whitespace
        self.skip_whitespace();
        match self.next() {
            Some(':') => {
                let start = self.index();
                self.skip_until(':');
                let end = self.index();
                match &self[start..end] {
                    ['t', 'o', 'c'] => unimplemented!(),
                    ['l', 'i', 's', 't'] => unimplemented!(),
                    ['t', 'a', 'b', 'l', 'e'] => unimplemented!(),
                    ['g', 'l', 'o', 's', 's'] => unimplemented!(),
                    _ => unimplemented!(),
                }
            }
            Some('#') => unimplemented!(),
            Some(_) => unimplemented!(),
            None => unimplemented!(),
        }
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

    /// Skips until the specified character is found.
    pub fn skip_until(&mut self, c: char) {
        while self.next() != Some(c) {}
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
    type Target = &'a[char];

    fn deref(&self) -> &&'a[char] {
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
