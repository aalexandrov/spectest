//! Utilities for reading [`MdDocument`] documents.

use pulldown_cmark::{Options, Parser};

use super::MdDocument;

impl<'input> MdDocument<'input> {
    /// Create an [`MdDocument`] from a `source` string.
    pub fn from_string(source: &'input str) -> Self {
        // Set up options and parser.
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        let md_reader = Parser::new_ext(&source, options);

        // Tokenize input
        let tokens = md_reader.into_offset_iter().collect::<Vec<_>>();

        Self { tokens }
    }
}
