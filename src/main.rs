extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate itertools;

mod document;
mod errors;
mod input;
mod parse;

use std::io;

fn main() -> errors::Result<()> {
    // for now, just read from stdin
    let stdin = io::stdin();
    let mut input = input::Input::new(stdin.lock());
    loop {
        let mut block = input.next_block()?;
        if block.len() == 0 {
            return Ok(());
        }
        println!("{:?}", block.parse()?);
    }
}
