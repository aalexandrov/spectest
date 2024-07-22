//! Core primitives and data types for BDD-like test definition and execution.
//!
//! The data model is inspired by [Gherkin][gherkin].
//!
//! [gherkin]: https://cucumber.io/docs/gherkin/reference/

use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::path::Path;

use pulldown_cmark::{CowStr, HeadingLevel};
use thiserror::Error;

use crate::core::reader::{sections, Pos};
use crate::md;

mod reader;

// Data model
// ==========

/// A spec section extracted from a parsed input.
///
/// These are modeled after [Gherkin's sections][gherkin].
///
/// [gherkin]: <https://cucumber.io/docs/gherkin/reference/>
#[derive(Debug)]
pub enum Section<'a, 'input> {
    Background(Background<'a>),
    Example(Example<'a, &'a mut CowStr<'input>>),
    Raw(Raw),
}

/// A `Background` spec section.
///
/// Modelled after [Gherkin's `Background` section][gherkin].
///
/// [gherkin]: <https://cucumber.io/docs/gherkin/reference/#background>
#[derive(Debug)]
pub struct Background<'a> {
    pub level: HeadingLevel,
    pub given: HashMap<&'a str, &'a str>,
}

/// An `Example` spec section.
///
/// Modelled after [Gherkin's `Example` section][gherkin].
///
/// [gherkin]: <https://cucumber.io/docs/gherkin/reference/#example>
#[derive(Debug)]
pub struct Example<'a, T = String> {
    pub level: HeadingLevel,
    pub name: &'a str,
    pub when: HashMap<&'a str, &'a str>,
    pub then: HashMap<&'a str, T>,
}

#[derive(Debug)]
pub struct Raw {
    level: HeadingLevel,
}

// Handler trait
// =============

/// A trait to be implemented by spec handlers.
pub trait Handler {
    type Error: Display;

    #[allow(unused)]
    fn enter(&mut self, background: &Background) -> Result<(), Self::Error> {
        Ok(()) // Ignore background sections by default.
    }

    #[allow(unused)]
    fn leave(&mut self, background: &Background) -> Result<(), Self::Error> {
        Ok(()) // Ignore background sections by default.
    }

    fn example(&mut self, example: &mut Example) -> Result<(), Self::Error>;
}

#[allow(async_fn_in_trait)]
/// An `async` version of [`Handler`].
pub trait AsyncHandler {
    type Error: Display;

    #[allow(unused)]
    async fn enter<'a>(&'a mut self, background: &'a Background<'a>) -> Result<(), Self::Error> {
        Ok(()) // Ignore background sections by default.
    }

    #[allow(unused)]
    async fn leave<'a>(&'a mut self, background: &'a Background<'a>) -> Result<(), Self::Error> {
        Ok(()) // Ignore background sections by default.
    }

    async fn example(&mut self, example: &mut Example) -> Result<(), Self::Error>;
}

/// Either [`process`] or [`rewrite`] the spec-style [`Sections`](Section)
/// extracted from a Markdown document at the given `path` using a user-defined
/// [`Handler`] depending on the value of the `REWRITE_SPECS` environment
/// variable.
///
/// If the `rewrite` flag is `true` the `path` is rewritten in order to reflect
/// the updated code snippets in the [`Example::then`] values.
pub fn run<P, H>(path: P, handler: &mut H)
where
    P: AsRef<Path>,
    H: Handler,
{
    let rewrite_specs = std::env::var("REWRITE_SPECS")
        .map(|var| !["false", "off", "0", ""].contains(&var.to_lowercase().as_ref()))
        .unwrap_or(false);

    let path_str = path.as_ref().to_str().unwrap_or("unknown");
    let result = if rewrite_specs {
        println!("rewriting spec at `{path_str}`");
        rewrite(path, handler)
    } else {
        println!("processing spec at `{path_str}`");
        process(path, handler)
    };

    if let Err(err) = result {
        panic!("{err}");
    }
}

/// An `async` version of `run`.
pub async fn async_run<P, H>(path: P, handler: &mut H)
where
    P: AsRef<Path>,
    H: AsyncHandler,
{
    let rewrite_specs = std::env::var("REWRITE_SPECS")
        .map(|var| !["false", "off", "0", ""].contains(&var.to_lowercase().as_ref()))
        .unwrap_or(false);

    let path_str = path.as_ref().to_str().unwrap_or("unknown");
    let result = if rewrite_specs {
        println!("rewriting spec at `{path_str}`");
        async_rewrite(path, handler).await
    } else {
        println!("processing spec at `{path_str}`");
        async_process(path, handler).await
    };

    if let Err(err) = result {
        panic!("{err}");
    }
}

/// Process spec-style [`Sections`](Section) extracted from a Markdown document
/// at the given `path` using a user-defined [`Handler`].
///
/// # Errors
///
/// - When the markdown reader encounters a malformed [`Section`].
/// - When the `handler` returns an error while processing a [`Section`].
/// - When the read or write process fails with a [`std::io::Error`].
pub fn process<P, H>(path: P, handler: &mut H) -> Result<(), Error<H::Error>>
where
    P: AsRef<Path>,
    H: Handler,
{
    // Read Markdown source into a String buffer.
    let md_source = std::fs::read_to_string(&path).expect("file");

    // Parse Markdown source.
    let mut md_doc = md::MdDocument::from_string(&md_source);

    const EMPTY_VEC: Vec<Background<'_>> = Vec::<Background>::new();
    let mut active = [EMPTY_VEC; HeadingLevel::H6 as usize - 1];

    // Iterate over spec-style sections in the parsed input.
    for section in sections(&mut md_doc) {
        let Ok(section) = section else {
            let err = section.unwrap_err().map_span(&md_source);
            return Err(err.into());
        };

        match section {
            Section::Background(background) => match handler.enter(&background) {
                Ok(()) => active[background.level as usize - 1].push(background),
                Err(err) => Err(Error::Handler(err))?,
            },
            Section::Example(example) => {
                let Example {
                    level,
                    name,
                    when,
                    then,
                } = example;

                if name.ends_with("(ignored)") {
                    continue;
                }

                let mut example = Example {
                    level,
                    name,
                    when,
                    then: then.iter().map(|(k, v)| (*k, v.to_string())).collect(),
                };

                let result = handler.example(&mut example);
                result.map_err(Error::<H::Error>::Handler)?;

                for (key, expect) in then.iter() {
                    let actual = example.then.get(key).expect("actual");
                    if expect.as_ref() != actual.as_str() {
                        return Err(Error::Failure {
                            key: key.to_string(),
                            example: name.to_string(),
                            expected: expect.to_string(),
                            actual: actual.to_string(),
                        });
                    }
                }
            }
            Section::Raw(section) => {
                for backgrounds in active[section.level as usize - 1..].iter_mut().rev() {
                    for background in backgrounds.drain(..).rev() {
                        let result = handler.leave(&background);
                        result.map_err(Error::Handler)?
                    }
                }
            }
        }
    }

    Ok(())
}

/// An `async` version of [`process`].
pub async fn async_process<P, H>(path: P, handler: &mut H) -> Result<(), Error<H::Error>>
where
    P: AsRef<Path>,
    H: AsyncHandler,
{
    // Read Markdown source into a String buffer.
    let md_source = std::fs::read_to_string(&path).expect("file");

    // Parse Markdown source.
    let mut md_doc = md::MdDocument::from_string(&md_source);

    const EMPTY_VEC: Vec<Background<'_>> = Vec::<Background>::new();
    let mut active = [EMPTY_VEC; HeadingLevel::H6 as usize - 1];

    // Iterate over spec-style sections in the parsed input.
    for section in sections(&mut md_doc) {
        let Ok(section) = section else {
            let err = section.unwrap_err().map_span(&md_source);
            return Err(err.into());
        };

        match section {
            Section::Background(background) => match handler.enter(&background).await {
                Ok(()) => active[background.level as usize - 1].push(background),
                Err(err) => Err(Error::Handler(err))?,
            },
            Section::Example(example) => {
                let Example {
                    level,
                    name,
                    when,
                    then,
                } = example;

                if name.ends_with("(ignored)") {
                    continue;
                }

                let mut example = Example {
                    level,
                    name,
                    when,
                    then: then.iter().map(|(k, v)| (*k, v.to_string())).collect(),
                };

                let result = handler.example(&mut example).await;
                result.map_err(Error::<H::Error>::Handler)?;

                for (key, expect) in then.iter() {
                    let actual = example.then.get(key).expect("actual");
                    if expect.as_ref() != actual.as_str() {
                        return Err(Error::Failure {
                            key: key.to_string(),
                            example: name.to_string(),
                            expected: expect.to_string(),
                            actual: actual.to_string(),
                        });
                    }
                }
            }
            Section::Raw(section) => {
                for backgrounds in active[section.level as usize - 1..].iter_mut().rev() {
                    for background in backgrounds.drain(..).rev() {
                        let result = handler.leave(&background).await;
                        result.map_err(Error::Handler)?
                    }
                }
            }
        }
    }

    Ok(())
}

/// Rewrite spec-style [`Sections`](Section) extracted from a Markdown document
/// at the given `path` using a user-defined [`Handler`].
///
/// # Errors
///
/// - When the markdown reader encounters a malformed [`Section`].
/// - When the `handler` returns an error while processing a [`Section`].
/// - When the read or write process fails with a [`std::io::Error`].
pub fn rewrite<P, H>(path: P, handler: &mut H) -> Result<(), Error<H::Error>>
where
    P: AsRef<Path>,
    H: Handler,
{
    // Read Markdown source into a String buffer.
    let md_source = std::fs::read_to_string(&path).expect("file");

    // Parse Markdown source.
    let mut md_doc = md::MdDocument::from_string(&md_source);

    const EMPTY_VEC: Vec<Background<'_>> = Vec::<Background>::new();
    let mut active = [EMPTY_VEC; HeadingLevel::H6 as usize - 1];

    // Iterate over spec-style sections in the parsed input.
    for section in sections(&mut md_doc) {
        let Ok(section) = section else {
            let err = section.unwrap_err().map_span(&md_source);
            return Err(err.into());
        };

        match section {
            Section::Background(background) => match handler.enter(&background) {
                Ok(()) => active[background.level as usize - 1].push(background),
                Err(err) => Err(Error::Handler(err))?,
            },
            Section::Example(example) => {
                let Example {
                    level,
                    name,
                    when,
                    mut then,
                } = example;

                if name.ends_with("(ignored)") {
                    continue;
                }

                let mut example = Example {
                    level,
                    name,
                    when,
                    then: then.iter().map(|(k, v)| (*k, v.to_string())).collect(),
                };

                let result = handler.example(&mut example);
                result.map_err(Error::<H::Error>::Handler)?;

                for (key, expect) in then.iter_mut() {
                    let actual = example.then.remove(key).expect("actual");
                    **expect = CowStr::from(actual);
                }
            }
            Section::Raw(section) => {
                for backgrounds in active[section.level as usize - 1..].iter_mut().rev() {
                    for background in backgrounds.drain(..).rev() {
                        let result = handler.leave(&background);
                        result.map_err(Error::Handler)?
                    }
                }
            }
        }
    }

    md_doc.write_to_path(&path)?;

    Ok(())
}

/// An `async` version of [`rewrite`].
pub async fn async_rewrite<P, H>(path: P, handler: &mut H) -> Result<(), Error<H::Error>>
where
    P: AsRef<Path>,
    H: AsyncHandler,
{
    // Read Markdown source into a String buffer.
    let md_source = std::fs::read_to_string(&path).expect("file");

    // Parse Markdown source.
    let mut md_doc = md::MdDocument::from_string(&md_source);

    const EMPTY_VEC: Vec<Background<'_>> = Vec::<Background>::new();
    let mut active = [EMPTY_VEC; HeadingLevel::H6 as usize - 1];

    // Iterate over spec-style sections in the parsed input.
    for section in sections(&mut md_doc) {
        let Ok(section) = section else {
            let err = section.unwrap_err().map_span(&md_source);
            return Err(err.into());
        };

        match section {
            Section::Background(background) => match handler.enter(&background).await {
                Ok(()) => active[background.level as usize - 1].push(background),
                Err(err) => Err(Error::Handler(err))?,
            },
            Section::Example(example) => {
                let Example {
                    level,
                    name,
                    when,
                    mut then,
                } = example;

                if name.ends_with("(ignored)") {
                    continue;
                }

                let mut example = Example {
                    level,
                    name,
                    when,
                    then: then.iter().map(|(k, v)| (*k, v.to_string())).collect(),
                };

                let result = handler.example(&mut example).await;
                result.map_err(Error::<H::Error>::Handler)?;

                for (key, expect) in then.iter_mut() {
                    let actual = example.then.remove(key).expect("actual");
                    **expect = CowStr::from(actual);
                }
            }
            Section::Raw(section) => {
                for backgrounds in active[section.level as usize - 1..].iter_mut().rev() {
                    for background in backgrounds.drain(..).rev() {
                        let result = handler.leave(&background).await;
                        result.map_err(Error::Handler)?
                    }
                }
            }
        }
    }

    md_doc.write_to_path(&path)?;

    Ok(())
}

// Errors
// ======

/// Errors that might be returned by a [`process`] call.
#[derive(Error, Debug)]
pub enum Error<H> {
    #[error("reader error: {0}")]
    SpecReader(#[from] reader::Error<Pos>),
    #[error("md writer error: {0}")]
    MdWriter(#[from] md::writer::Error),
    #[error("handler error: {0}")]
    Handler(H),
    #[error("unexpected `{key}` in {example}\n# Expected:\n{expected}\n# Actual:\n{actual}")]
    Failure {
        key: String,
        example: String,
        expected: String,
        actual: String,
    },
    #[error("io error")]
    IO(#[from] std::io::Error),
    #[error("unknown error")]
    Unknown(String),
}

#[cfg(test)]
mod tests {
    use crate::core::rewrite;

    use super::examples::*;
    use super::{process, Background, Example, Handler};

    #[test]
    fn test_process() -> std::io::Result<()> {
        struct TestHandler;

        impl Handler for TestHandler {
            type Error = String;

            fn enter(&mut self, _background: &Background) -> Result<(), Self::Error> {
                Ok(())
            }

            fn leave(&mut self, _background: &Background) -> Result<(), Self::Error> {
                Ok(())
            }

            fn example(&mut self, example: &mut Example) -> Result<(), Self::Error> {
                if let Some(code) = example.then.get_mut("output") {
                    *code = String::from(OUTPUT_SQL);
                }
                Ok(())
            }
        }

        let path = write_spec(&make_spec(INPUT_SQL, OUTPUT_SQL))?;

        process(path, &mut TestHandler).expect("`process` call completes cleanly");

        Ok(())
    }

    #[test]
    fn test_rewrite() -> std::io::Result<()> {
        struct TestHandler;

        impl Handler for TestHandler {
            type Error = String;

            fn enter(&mut self, _background: &Background) -> Result<(), Self::Error> {
                Ok(())
            }

            fn leave(&mut self, _background: &Background) -> Result<(), Self::Error> {
                Ok(())
            }

            fn example(&mut self, example: &mut Example) -> Result<(), Self::Error> {
                if let Some(code) = example.then.get_mut("output") {
                    *code = String::from("<redacted>\n");
                }
                Ok(())
            }
        }

        let path = write_spec(&make_spec(INPUT_SQL, OUTPUT_SQL))?;

        rewrite(&path, &mut TestHandler).expect("`rewrite` call completes cleanly");

        let exp = make_spec(INPUT_SQL, "<redacted>");
        let act = std::fs::read_to_string(&path)?;

        assert_eq!(act, exp);

        Ok(())
    }
}

#[cfg(test)]
pub mod examples {
    use std::io::Write;

    use tempfile::{NamedTempFile, TempPath};

    pub fn make_spec(input: &str, output: &str) -> String {
        [
            "
            # Feature: SQL formatting

            Spec for an opinionated SQL formatter.

            ## Background

            Given `pipeline` as:

            ```rust
            let output = display(ast_to_ast(parse(input)));
            ```

            _Note_: this is just for readability.

            And `environment` as:

            ```sql
            CREATE TABLE s(x int, y int);
            CREATE TABLE t(y int, z int);
            ```

            ## Example: Simple queries

            When `input` is:

            ```sql
            ",
            input.trim(),
            "
            ```

            Then `output` is:

            ```sql
            ",
            output.trim(),
            "
            ```
        ",
        ]
        .into_iter()
        .fold(String::new(), |a, b| a + &textwrap::dedent(b))
        .trim_start()
        .to_string()
    }

    pub const INPUT_SQL: &str = indoc::indoc! {r"
        SELECT x, y, z FROM s JOIN t USING(y);
    "};

    pub const OUTPUT_SQL: &str = indoc::indoc! {r"
        SELECT
            x, y, z
        FROM
            s
            JOIN t USING(y);
    "};

    pub fn write_spec(spec: &str) -> Result<TempPath, std::io::Error> {
        // Create a file inside of `std::env::temp_dir()`.
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(spec.as_bytes())?;
        Ok(temp_file.into_temp_path())
    }
}
