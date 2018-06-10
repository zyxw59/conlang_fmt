use document;
use errors::{Error, ErrorKind};
use input;

struct Parser {
    document: document::Document,
}

impl Parser {
    pub fn new() -> Parser {
        Parser {
            document: Default::default(),
        }
    }

    fn block(&self, mut block: input::Block) -> Result<Option<document::Block>, Error> {
        // skip leading whitespace
        block.skip_whitespace();
        match block.next() {
            Some(':') => {
                let start = block.index();
                block.skip_until(':');
                let end = block.index();
                match &block[start..end] {
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
}
