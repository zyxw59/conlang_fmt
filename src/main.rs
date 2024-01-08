#[macro_use]
mod html;
mod blocks;
mod document;
mod errors;
mod input;
mod parse;
mod text;

use std::io;

use errors::Result as EResult;

fn main() {
    if let Err(e) = main_result() {
        for err in e.chain() {
            eprintln!("{err}");
        }
    }
}

fn main_result() -> EResult<()> {
    // for now, just read from stdin
    let stdin = io::stdin();
    let mut input = input::Input::new(stdin.lock());
    let mut document: document::Document = Default::default();
    loop {
        let block = input.next_block()?;
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
