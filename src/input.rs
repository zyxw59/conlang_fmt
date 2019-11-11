use std::io::{BufRead, Lines};
use std::iter::Enumerate;

use failure::ResultExt;

use crate::errors::{ErrorKind, Result as EResult};
use crate::parse::Block;

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
            let line = line.with_context(|e| ErrorKind::input_error(e, line_number))?;
            // blank lines
            if line.trim().is_empty() {
                // if the buffer is empty, don't return anything
                if !self.buffer.is_empty() {
                    // but if it's not, we've reached the end of a block
                    break;
                }
            } else {
                if self.buffer.is_empty() {
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
        "#
        .as_bytes();

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

        "#
        .as_bytes();

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
        block 1, line 3"#
            .as_bytes();

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
