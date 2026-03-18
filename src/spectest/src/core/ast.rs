//! AST support for BDD-files represented as Markdown.
#![allow(unused)]

use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Write;
use std::io;
use std::path::Path;

use markdown_ast as mdast;
use pulldown_cmark_to_cmark::cmark_with_options;

use crate::md::writer::Error;

/// A parsed version of a Markdown source.
///
/// The struct is opaque and encapsulates the result of parsing at the Markdown
/// level. See the contents of [`crate::spec`] for extracting sections from an
/// [`MdDocument`] instance.
pub struct SpecTestDocument {
    pub(crate) ast: Vec<mdast::Block>,
    pub(crate) sections: Vec<Section>,
}

/// A spec section extracted from a [`SpecTestDocument`].
///
/// These are modeled after [Gherkin's sections][gherkin].
///
/// [gherkin]: <https://cucumber.io/docs/gherkin/reference/>
#[derive(Debug)]
pub enum Section {
    Background(Background),
    Example(Example),
}

/// A [`Background`] spec section.
///
/// Modelled after [Gherkin's `Background` section][gherkin].
///
/// [gherkin]: <https://cucumber.io/docs/gherkin/reference/#background>
#[derive(Debug)]
pub struct Background {
    /// A transient UUID used to generate the surrogate keys to replace the
    /// original values when we etract the background contents from the
    /// document.
    pub uuid: uuid::Uuid,
    pub level: mdast::HeadingLevel,
    pub given: BTreeMap<String, String>,
}

/// An [`Example`] spec section.
///
/// Modelled after [Gherkin's `Example` section][gherkin].
///
/// [gherkin]: <https://cucumber.io/docs/gherkin/reference/#example>
#[derive(Debug)]
pub struct Example {
    /// A transient UUID used to generate the surrogate keys to replace the
    /// original values when we etract the background contents from the
    /// document.
    pub uuid: uuid::Uuid,
    pub level: mdast::HeadingLevel,
    pub name: String,
    pub when: BTreeMap<String, String>,
    pub then: BTreeMap<String, String>,
}

impl SpecTestDocument {
    pub fn new(mut ast: Vec<mdast::Block>) -> Self {
        let mut sections = Vec::new();
        for block in ast.iter_mut() {
            // extract background section
            // if let Ok()
        }

        Self { ast, sections }
    }

    /// Create an [`MdDocument`] from a `source` string.
    pub fn read_from_string(source: &str) -> Self {
        let events = mdast::markdown_to_events(source);
        let ast = mdast::events_to_ast(events);

        Self::new(ast)
    }

    pub fn read_from_file<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let source = std::fs::read_to_string(path)?;
        Ok(Self::read_from_string(&source))
    }

    /// Consume a [`SpecTestDocument`] and write it back into a [`String`].
    pub fn write_to_string(self) -> Result<String, Error> {
        let options = default_to_markdown_options();

        let mut output = String::new();
        {
            let events = mdast::ast_to_events(&self.ast).into_iter();
            cmark_with_options(events, &mut output, options)?;
        }
        output.push('\n');

        Ok(output)
    }

    /// Consume a [`SpecTestDocument`] and write it back into the given `path`.
    pub fn write_to_path<P>(self, path: P) -> Result<(), Error>
    where
        P: AsRef<Path>,
    {
        let options = default_to_markdown_options();

        let mut output = IoToFmtWrite(std::fs::File::create(&path)?);
        {
            let events = mdast::ast_to_events(&self.ast).into_iter();
            cmark_with_options(events, &mut output, options)?;
        }
        output.write_char('\n')?;

        Ok(())
    }
}

fn default_to_markdown_options() -> pulldown_cmark_to_cmark::Options<'static> {
    pulldown_cmark_to_cmark::Options {
        code_block_token_count: 3,
        emphasis_token: '_',
        ..pulldown_cmark_to_cmark::Options::default()
    }
}

struct IoToFmtWrite<W: io::Write>(W);

impl<W: io::Write> fmt::Write for IoToFmtWrite<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

#[cfg(test)]
mod roundtrip_tests {
    use spectest_macros::glob_test;

    use super::*;

    #[glob_test("testdata/md_writer/**/*.md")]
    fn test(path: &str) {
        let md_src = std::fs::read_to_string(path).expect("source string");
        let md_doc = SpecTestDocument::read_from_string(&md_src);
        let md_out = md_doc.write_to_string().expect("output string");

        // println!("---");
        // for (event, _span) in md_doc.tokens.iter() {
        //     println!("span={_span:03?} - event={event:?}");
        // }
        // println!("---");

        assert_eq!(&md_src, &md_out)
    }
}
