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

    fn block(&self, block: input::Block) -> Result<Option<document::Block>, Error> {
        let mut block = block.peekable();
        let first = match block.peek() {
            Some(first) => first.trim(),
            None => return Ok(None),
        };
        let mut it = first.bytes();
        match it.next() {
            Some(b':') => {
                let mut len = 0;
                while it.next() != Some(b':') {
                    len += 1;
                }
                match &first[1..len + 1] {
                    "toc" => unimplemented!(),
                    "list" => unimplemented!(),
                    "table" => unimplemented!(),
                    "gloss" => unimplemented!(),
                    _ => unimplemented!(),
                }
            }
            Some(b'#') => unimplemented!(),
            Some(_) => unimplemented!(),
            None => unimplemented!(),
        }
    }
}
