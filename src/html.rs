use std::fmt;

/// A structure which when formatted entity-encodes a minimal set of characters:
///
/// - `"` => `&quot;`
/// - `&` => `&amp;`
/// - `'` => `&#x27;`
/// - `<` => `&lt;`
/// - `>` => `&gt;`
pub struct Encoder<'a>(pub &'a str);

impl fmt::Display for Encoder<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for c in self.0.chars() {
            if let Some(ent) = get_entity(c) {
                write!(f, "&{};", ent)?;
            } else {
                write!(f, "{}", c)?;
            }
        }
        Ok(())
    }
}

fn get_entity(c: char) -> Option<&'static str> {
    match c {
        '"' => Some("quot"),
        '&' => Some("amp"),
        '\'' => Some("#x27"),
        '<' => Some("lt"),
        '>' => Some("gt"),
        _ => None,
    }
}
