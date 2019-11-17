mod blocks;
mod document;
mod errors;
mod input;
mod parse;
mod text;

use failure::Fail;
use std::io;

use errors::Result as EResult;

fn main() {
    if let Err(e) = main_result() {
        print_errors(&e);
    }
}

fn main_result() -> EResult<()> {
    // for now, just read from stdin
    let stdin = io::stdin();
    let mut input = input::Input::new(stdin.lock());
    let mut document: document::Document = Default::default();
    loop {
        let mut block = input.next_block()?;
        if let Some(block) = block.parse()? {
            document.add_block(block)?;
        } else {
            break;
        }
    }
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    document.write(&mut stdout)
}

fn print_errors(e: &dyn Fail) {
    for c in e.iter_chain() {
        println!("{}", c);
    }
}
