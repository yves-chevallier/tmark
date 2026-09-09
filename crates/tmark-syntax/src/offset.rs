//! Offsets of re-parsed content back to the source file.
//!
//! A container body, an admonition body or a definition body is collected
//! by the tokenizer as one string with its prefixes stripped, then parsed
//! again as a document. Positions in that parse are offsets into the
//! collected string; the `stops` recorded by the tokenizer say where each
//! slice came from. An [`OffsetMap`] translates one into the other and
//! chains through nested re-parses.

use std::rc::Rc;

/// Maps offsets in a re-parsed string to offsets in its parent text.
#[derive(Debug, Default)]
pub struct OffsetMap {
    /// `(offset in the string, offset in the parent)` for every slice start,
    /// sorted by the first element.
    stops: Vec<(usize, usize)>,
    /// The map of the parent text, when the parent was itself re-parsed.
    parent: Option<Rc<OffsetMap>>,
}

impl OffsetMap {
    /// The identity map: the text *is* the source file.
    pub fn identity() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// A map for content collected with `stops`, inside `parent`.
    pub fn nested(stops: Vec<(usize, usize)>, parent: Rc<OffsetMap>) -> Rc<Self> {
        Rc::new(Self {
            stops,
            parent: Some(parent),
        })
    }

    /// Source offset of `offset` in the mapped text.
    pub fn translate(&self, offset: usize) -> usize {
        let local = if self.stops.is_empty() {
            offset
        } else {
            let index = match self.stops.binary_search_by(|(at, _)| at.cmp(&offset)) {
                Ok(index) => index,
                Err(0) => 0,
                Err(index) => index - 1,
            };
            let (at, source) = self.stops[index];
            source + (offset - at)
        };
        match &self.parent {
            Some(parent) => parent.translate(local),
            None => local,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_identity() {
        assert_eq!(OffsetMap::identity().translate(42), 42);
    }

    #[test]
    fn nested_maps_chain() {
        // "ab\ncd" collected from source offsets 10.. and 20..
        let outer = OffsetMap::nested(vec![(0, 10), (3, 20)], OffsetMap::identity());
        assert_eq!(outer.translate(0), 10);
        assert_eq!(outer.translate(1), 11);
        assert_eq!(
            outer.translate(2),
            12,
            "the newline maps just after the slice"
        );
        assert_eq!(outer.translate(3), 20);
        assert_eq!(outer.translate(4), 21);
        // Content "d" re-parsed inside the outer content at offset 4.
        let inner = OffsetMap::nested(vec![(0, 4)], outer);
        assert_eq!(inner.translate(0), 21);
    }
}
