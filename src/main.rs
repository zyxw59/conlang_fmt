extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate htmlescape;
extern crate itertools;

mod document;
mod errors;
mod input;
mod output;
mod parse;

use std::io;
use failure::Fail;

fn main() {
    // for now, just read from stdin
    let stdin = io::stdin();
    let mut input = input::Input::new(stdin.lock());
    loop {
        let mut block = match input.next_block() {
            Err(e) => {
                print_errors(e);
                continue;
            }
            Ok(block) => block,
        };
        if block.len() == 0 {
            return;
        }
        match block.parse() {
            Err(e) => {
                print_errors(e);
                continue;
            }
            Ok(block) => println!("{:?}", block),
        }
    }
}

fn print_errors(e: impl Fail) {
    for c in e.causes() {
        println!("{}", c);
    }
}
