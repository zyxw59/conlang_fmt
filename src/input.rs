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
    pub fn next_block(&mut self) -> Result<Drain<String>, Error> {
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
        Ok(self.buffer.drain(..))
    }
}
