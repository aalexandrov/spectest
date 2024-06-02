//! Support for BDD-files written in Markdown.

pub(crate) mod reader;
pub(crate) mod writer;

use crate::Token;

/// A parsed version of a Markdown source.
///
/// The struct is opaque and encapsulates the result of parsing at the Markdown
/// level. See the contents of [`crate::spec`] for extracting sections from an
/// [`MdDocument`] instance.
pub struct MdDocument<'input> {
    pub(crate) tokens: Vec<Token<'input>>,
}

#[cfg(test)]
mod roundtrip_tests {
    use spectest_macros::glob_test;

    use crate::md;

    #[glob_test("testdata/md_writer/**/*.md")]
    fn test(path: &str) {
        let md_src = std::fs::read_to_string(path).expect("source string");
        let md_doc = md::MdDocument::from_string(&md_src);
        let md_out = md_doc.write_to_string().expect("output string");

        // println!("---");
        // for (event, _span) in md_doc.tokens.iter() {
        //     println!("span={_span:03?} - event={event:?}");
        // }
        // println!("---");

        assert_eq!(&md_src, &md_out)
    }
}
