use std::collections::HashMap;
use std::io::{Result as IoResult, Write};

use crate::blocks::{BlockCommon, BlockType, Parameter};
use crate::document::Document;
use crate::errors::{ErrorKind, Result as EResult};
use crate::text::Text;

type OResult<T> = EResult<Option<T>>;

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Replacements {
    pub replacements: HashMap<String, Text>,
}

impl Replacements {
    pub fn new() -> Replacements {
        Default::default()
    }

    /// Inserts the given key/value pair, returning an error if the key is already present.
    pub fn insert(&mut self, key: String, value: Text) -> EResult<()> {
        // using `HashMap::entry` here moves `key`, so it can't be used in the error.
        #[allow(clippy::map_entry)]
        if self.replacements.contains_key(&key) {
            Err(ErrorKind::Replace(key).into())
        } else {
            self.replacements.insert(key, value);
            Ok(())
        }
    }

    /// Updates `self` with keys from `other`, replacing duplicates.
    pub fn update(&mut self, other: &mut Replacements) {
        for (k, v) in other.drain() {
            self.replacements.insert(k, v);
        }
    }

    fn drain(&mut self) -> impl Iterator<Item = (String, Text)> + '_ {
        self.replacements.drain()
    }

    /// Gets the given key.
    pub fn get(&self, key: &str) -> Option<&Text> {
        self.replacements.get(key)
    }
}

impl BlockType for Replacements {
    fn write(&self, _: &mut dyn Write, _: &BlockCommon, _: &Document) -> IoResult<()> {
        Ok(())
    }

    fn update_param(&mut self, param: Parameter) -> OResult<Parameter> {
        Ok(Some(param))
    }

    fn as_mut_replacements(&mut self) -> Option<&mut Replacements> {
        Some(self)
    }
}
