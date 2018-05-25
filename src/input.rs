use std::io::{BufRead, Lines};
use std::vec::Drain;

use failure::ResultExt;

use errors::{Error, ErrorKind};

pub struct Input<B> {
    lines: Lines<B>,
    line_number: usize,
    buffer: Vec<String>,
}

impl<B> Input<B> where B: BufRead {
    pub fn new(input: B) -> Input<B> {
        Input {
            lines: input.lines(),
            line_number: 0,
            buffer: Vec::new(),
        }
    }

    /// Retrieves the next block from the input, as an iterator over lines.
    ///
    /// Blocks are delimited by blank (all-whitespace) lines.
    ///
    /// An empty block signifies that the end of the input has been reached.
    pub fn next_block(&mut self) -> Result<Block, Error> {
        while let Some(line) = self.lines.next() {
            // unwrap line
            let line = line.with_context(|e| ErrorKind::from_io(e, self.line_number))?;
            self.line_number += 1;
            // blank lines
            if line.trim().len() == 0 {
                // if the buffer is empty, don't return anything
                if self.buffer.len() > 0 {
                    // but if it's not, we've reached the end of a block
                    break;
                }
            } else {
                // otherwise push the line into the block
                self.buffer.push(line);
            }
        }
        // if we broke earlier, or if we've reached the end of the text, return the iterator.
        // we use `drain` so that we can reuse `buffer`.
        Ok(Block {
            len: self.buffer.len(),
            start: self.line_number - self.buffer.len(),
            iter: self.buffer.drain(..),
        })
    }
}

/// An iterator over the lines of a block.
pub struct Block<'a> {
    iter: Drain<'a, String>,
    len: usize,
    start: usize,
}

impl<'a> Block<'a> {
    /// Returns the length of the block, in number of lines.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the starting line number of the block.
    pub fn start(&self) -> usize {
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
