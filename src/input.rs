use std::io::{BufRead, Lines};
use std::iter::Enumerate;
use std::vec::Drain;

use failure::ResultExt;

use errors::{Error, ErrorKind};

#[derive(Debug)]
pub struct Input<B> {
    lines: Enumerate<Lines<B>>,
    buffer: Vec<String>,
}

impl<B> Input<B> where B: BufRead {
    pub fn new(input: B) -> Input<B> {
        Input {
            lines: input.lines().enumerate(),
            buffer: Vec::new(),
        }
    }

    /// Retrieves the next block from the input, as an iterator over lines.
    ///
    /// Blocks are delimited by blank (all-whitespace) lines.
    ///
    /// An empty block signifies that the end of the input has been reached.
    pub fn next_block(&mut self) -> Result<Block, Error> {
        let mut start_line = None;
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
                self.buffer.push(line);
            }
        }
        // if we broke earlier, or if we've reached the end of the text, return the iterator.
        // we use `drain` so that we can reuse `buffer`.
        Ok(Block {
            len: self.buffer.len(),
            start: start_line,
            iter: self.buffer.drain(..),
        })
    }
}

/// An iterator over the lines of a block.
#[derive(Debug)]
pub struct Block<'a> {
    iter: Drain<'a, String>,
    len: usize,
    start: Option<usize>,
}

impl<'a> Block<'a> {
    /// Returns the length of the block, in number of lines.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the starting line number of the block, which is only defined for non-empty blocks.
    pub fn start(&self) -> Option<usize> {
        self.start
    }
}

impl<'a> Iterator for Block<'a> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
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
            assert_eq!(block.len(), 3);
            assert_eq!(block.start(), Some(0));
        }
        {
            let block = input.next_block().unwrap();
            assert_eq!(block.len(), 2);
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
            assert_eq!(block.len(), 3);
            assert_eq!(block.start(), Some(0));
        }
        {
            let block = input.next_block().unwrap();
            assert_eq!(block.len(), 2);
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
            assert_eq!(block.len(), 3);
            assert_eq!(block.start(), Some(0));
        }
        {
            let block = input.next_block().unwrap();
            assert_eq!(block.len(), 0);
            assert_eq!(block.start(), None);
        }
    }
}
