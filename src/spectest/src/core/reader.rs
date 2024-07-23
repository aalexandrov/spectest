use std::collections::HashMap;
use std::fmt::Display;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;

use fs2::FileExt;
use pulldown_cmark::{CowStr, Event, HeadingLevel};
use thiserror::Error;

use crate::md::MdDocument;
use crate::{event, span, Token, Tokens};

use super::{Background, Example, Raw, Section};

/// Read file contents into a String using a shared lock.
pub fn read_to_string<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    let mut file_buff = String::new();

    let mut file = OpenOptions::new().read(true).open(&path)?;
    file.lock_shared()?;
    file.read_to_string(&mut file_buff)?;

    Ok(file_buff)
}

// Sections iterators
// ==================

/// Iterate over the [`Sections`](Section) contained in a [`MdDocument`].
///
/// The `input` parameter is a mutable reference because [`Example`] sections
/// bind their `then` values to the original [`CowStr`] event of the backing
/// document. This allows the [`crate::spec::process`] function to handle
/// rewrite requests.
pub fn sections<'a, 'input>(input: &'a mut MdDocument<'input>) -> SectionsIter<'a, 'input> {
    SectionsIter {
        tokens: &mut input.tokens[..],
    }
}

/// An iterator over the [`Sections`](Section) contained in a [`MdDocument`].
///
/// See [`sections`] for details.
pub struct SectionsIter<'a, 'input> {
    tokens: Tokens<'a, 'input>,
}

impl<'a, 'input> Iterator for SectionsIter<'a, 'input> {
    type Item = Result<Section<'a, 'input>, Error<usize>>;

    fn next(&mut self) -> Option<Self::Item> {
        while advance::section(&mut self.tokens) {
            let Some(section) = expect::section(&mut self.tokens) else {
                continue;
            };
            if Background::check_header(section) {
                let section = Background::try_from(section);
                return Some(section.map(Section::Background));
            } else if Example::check_header(section) {
                let section = Example::try_from(section);
                return Some(section.map(Section::Example));
            } else {
                let section = Raw::from(section);
                return Some(Ok(Section::Raw(section)));
            };
        }

        None
    }
}

// Section from Token slice constructors
// =====================================

impl<'a> Background<'a> {
    /// Check if the section header starting with the `Background` string.
    fn check_header<'input>(section: &'a mut [Token<'input>]) -> bool {
        use pulldown_cmark::{CowStr::*, Event::*};

        if let Some((Text(Borrowed(heading)), _)) = section.get(1) {
            heading.starts_with("Background")
        } else {
            unreachable!("Asserted by `TokenSlice::next_section()`")
        }
    }

    fn try_from<'input>(section: &'a mut [Token<'input>]) -> Result<Self, Error<usize>> {
        use pulldown_cmark::Event::*;

        let level = util::heading_level(section);

        // Skip the section header.
        let (heading, mut body) = section.split_at_mut(3);

        let mut given = HashMap::<&'a str, &'a str>::new();
        while !body.is_empty() {
            let mut pos = span(&body[0]).start;
            if let Some(key) = {
                // Debug detected slice:
                // crate::debug("background:given:key", body);

                if advance::paragraph(&mut body) {
                    pos = span(&body[0]).start;
                }
                expect::paragraph(&mut body, |p| util::is_given(p, given.is_empty())).transpose()?
            } {
                // Debug detected slice:
                // crate::debug("background:given:val", body);

                let val = expect::code_block(&mut body, |c| match c {
                    [(Text(val), _span)] => Ok(val),
                    _ => Err(Error::ExpectedCode { pos }),
                })?;

                given.insert(key, val);
            }
        }

        if given.is_empty() {
            let pos = span(&heading[0]).start;
            return Err(Error::MissingWhen { pos });
        }

        Ok(Self { level, given })
    }
}

impl<'a, 'input> Example<'a, &'a mut CowStr<'input>> {
    /// Check if the section header starting with the `Example` string.
    fn check_header(section: &'a mut [Token<'input>]) -> bool {
        use pulldown_cmark::{CowStr::*, Event::*};

        if let Some((Text(Borrowed(heading)), _)) = section.get(1) {
            heading.starts_with("Example:")
        } else {
            unreachable!("Asserted by `TokenSlice::next_section()`")
        }
    }

    fn try_from(section: &'a mut [Token<'input>]) -> Result<Self, Error<usize>> {
        use pulldown_cmark::{CowStr::*, Event::*};

        let (heading, mut body) = section.split_at_mut(3);

        let level = util::heading_level(heading);

        let Some((Text(Borrowed(name)), _)) = heading.get(1) else {
            unreachable!("Asserted by `TokenSlice::next_section()`")
        };

        let mut when = HashMap::<&'a str, &'a str>::new();
        while !body.is_empty() {
            let mut pos = span(&body[0]).start;
            if let Some(key) = {
                // Debug detected slice:
                // crate::debug("example:when:key", body);

                if advance::paragraph(&mut body) {
                    pos = span(&body[0]).start;
                }
                if body.len() >= 5 && util::is_then(&mut body[1..4], true).is_some() {
                    break;
                }
                expect::paragraph(&mut body, |p| util::is_when(p, when.is_empty())).transpose()?
            } {
                // Debug detected slice:
                // crate::debug("example:when:key", body);

                let val = expect::code_block(&mut body, |c| match c {
                    [(Text(val), _span)] => Ok(val),
                    _ => Err(Error::ExpectedCode { pos }),
                })?;

                when.insert(key, val);
            }
        }

        let mut then = HashMap::<&'a str, &'a mut CowStr<'input>>::new();
        while !body.is_empty() {
            let mut pos = span(&body[0]).start;
            if let Some(key) = {
                // Debug detected slice:
                // crate::debug("example:then:key", body);

                if advance::paragraph(&mut body) {
                    pos = span(&body[0]).start;
                }
                expect::paragraph(&mut body, |p| util::is_then(p, then.is_empty())).transpose()?
            } {
                // Debug detected slice:
                // crate::debug("example:then:val", body);

                let val = expect::code_block(&mut body, |c| match c {
                    [(Text(val), _span)] => Ok(val),
                    _ => Err(Error::ExpectedCode { pos }),
                })?;

                then.insert(key, val);
            }
        }

        if when.is_empty() {
            let pos = span(&heading[0]).start;
            return Err(Error::MissingWhen { pos });
        }
        if then.is_empty() {
            let pos = span(&heading[0]).start;
            return Err(Error::MissingThen { pos });
        }

        Ok(Self {
            level,
            name,
            when,
            then,
        })
    }
}

impl Raw {
    fn from(section: &mut [Token<'_>]) -> Self {
        Self {
            level: util::heading_level(section),
        }
    }
}

mod advance {
    use super::*;

    /// Find the next heading start tag, consuming everything before that.
    pub(super) fn section(tokens: &mut Tokens<'_, '_>) -> bool {
        use pulldown_cmark::{Event::*, Tag as S};

        let mut finger = 0;

        let result = util::advance(tokens, &mut finger, |token| {
            matches!(event(token), Start(S::Heading { .. }))
        });

        util::take_mut(tokens, finger);

        result
    }

    /// Find the next paragraph start tag, consuming everything before that.
    pub(super) fn paragraph(tokens: &mut Tokens<'_, '_>) -> bool {
        use pulldown_cmark::{Event::*, Tag as S};

        let mut finger = 0;

        let result = util::advance(tokens, &mut finger, |token| {
            matches!(event(token), Start(S::Paragraph))
        });

        util::take_mut(tokens, finger);

        result
    }
}

mod expect {
    use super::*;

    /// Assert that the `tokens` sequence is non-empty and starts with a
    /// `Heading` start tag. Consume and return a mutable slice that includes
    /// everything until the next heading or the end of the sequence.
    pub(super) fn section<'a, 'input>(
        tokens: &mut Tokens<'a, 'input>,
    ) -> Option<Tokens<'a, 'input>> {
        use pulldown_cmark::{Event::*, Tag as S};

        let mut finger = 0;

        // Assert that we are at a section start, returning elsewhere.
        if tokens.is_empty() {
            return None;
        }
        let Some(Start(S::Heading { .. })) = tokens.first().map(event) else {
            return None;
        };

        // Record current position as start and advance the finger.
        let start = finger;
        finger += 1;

        // Advance the finger to the start of the next section.
        util::advance(tokens, &mut finger, |token| {
            matches!(event(token), Start(S::Heading { .. }))
        });

        let paragraph = util::take_mut(tokens, finger);

        // Return the result.
        Some(&mut paragraph[start..])
    }

    /// Find the next paragraph that matches the given `predicate`, consuming
    /// everything before that, and return the `predicate` result if not `None`.
    pub(super) fn paragraph<'a, 'input, T, P>(
        tokens: &mut Tokens<'a, 'input>,
        predicate: P,
    ) -> Option<T>
    where
        P: Fn(Tokens<'a, 'input>) -> Option<T>,
    {
        use pulldown_cmark::{Event::*, Tag as S, TagEnd as E};

        let mut finger = 0;

        // Assert that we are at a paragraph start, returning elsewhere.
        if tokens.is_empty() {
            return None;
        }
        let Some(Start(S::Paragraph)) = tokens.first().map(event) else {
            return None;
        };

        // Record current position as start and advance the finger.
        let start = finger;
        finger += 1;

        // Advance the finger to the end of the current paragraph, asserting
        // that we have stopped at an the corresponding closing tag.
        if !util::advance(tokens, &mut finger, |token| {
            matches!(event(token), End(E::Paragraph))
        }) {
            unreachable!("token stream is not well-formed (missing closing paragraph tag)");
        }

        // Record current position as end and advance the finger.
        let end = finger;
        finger += 1;

        // Debug detected slice
        // crate::debug("paragraph", &mut tokens[start..=end]);

        let paragraph = util::take_mut(tokens, finger);

        predicate(&mut paragraph[start + 1..=end - 1])
    }

    /// Consume a code block.
    pub(super) fn code_block<'a, 'input, T, P>(
        tokens: &mut Tokens<'a, 'input>,
        predicate: P,
    ) -> Result<T, Error<usize>>
    where
        P: Fn(Tokens<'a, 'input>) -> Result<T, Error<usize>>,
    {
        use pulldown_cmark::{Event::*, Tag as S, TagEnd as E};

        let mut finger = 0;

        // Ensure that the finger is the start of the next code block,
        // returning immediately elsewhere.
        let Some(Start(S::CodeBlock(_))) = tokens.get(finger).map(event) else {
            // This is a bit hacky: in this corner case we call the predicate
            // function with an empty slice and rely that it will return an Err.
            let code = util::take_mut(tokens, finger);
            return predicate(code);
        };

        // Record current position as start and advance the finger.
        let start = finger;
        finger += 1;

        // Advance the finger to the end of the current code block, asserting
        // that we have stopped at an the corresponding closing tag.
        if !util::advance(tokens, &mut finger, |token| {
            matches!(event(token), End(E::CodeBlock))
        }) {
            unreachable!("token stream is not well-formed (missing closing paragraph tag)");
        }

        // Record current position as end and advance the finger.
        let end = finger;
        finger += 1;

        // Debug detected slice
        // crate::debug("code", &mut tokens[start..=end]);

        let code = util::take_mut(tokens, finger);

        predicate(&mut code[start + 1..=end - 1])
    }
}

mod util {
    use super::*;

    pub(crate) fn heading_level(section: Tokens<'_, '_>) -> HeadingLevel {
        use pulldown_cmark::{Event::*, Tag as S};

        if let Some((Start(S::Heading { level, .. }), _)) = section.first() {
            *level
        } else {
            unreachable!("Asserted by `TokenSlice::next_section()`")
        }
    }

    /// Removes the subslice corresponding to the given range and returns a
    /// mutable reference to it.
    ///
    /// We need this because [`take_mut`][rust_take_mut] is still unstable.
    ///
    /// [rust_take_mut]: <https://doc.rust-lang.org/std/primitive.slice.html#method.take_mut>
    pub(crate) fn take_mut<'a, 'input>(
        tokens: &mut Tokens<'a, 'input>,
        position: usize,
    ) -> Tokens<'a, 'input> {
        let (lhs, rhs) = std::mem::take(tokens).split_at_mut(position);
        *tokens = rhs;
        lhs
    }

    /// Advance the `finger` reference to the first token in `tokens` that
    /// matches the given `predicate` or `tokens.len()` otherwise.
    ///
    /// Return `true` iff a match was found (i.e., iff `finger < tokens.len()`
    /// after the `advance` call).
    pub(crate) fn advance<'a, 'input, P>(
        tokens: Tokens<'a, 'input>,
        finger: &mut usize,
        predicate: P,
    ) -> bool
    where
        P: Fn(&Token<'input>) -> bool,
    {
        // Advance the finger to the start of the next section.
        while let Some(token) = tokens.get(*finger) {
            if predicate(token) {
                break;
            } else {
                *finger += 1;
            }
        }

        *finger < tokens.len()
    }

    pub(crate) fn starts_with(tokens: &[Token<'_>], pat: &str) -> bool {
        match tokens.first().map(event) {
            Some(Event::Text(t)) => t.starts_with(pat),
            _ => false,
        }
    }

    pub(crate) fn ends_with(tokens: &[Token<'_>], pat: &str) -> bool {
        match tokens.last().map(event) {
            Some(Event::Text(t)) => t.ends_with(pat),
            _ => false,
        }
    }

    pub(crate) fn is_given<'a, 'input>(
        paragraph: Tokens<'a, 'input>,
        first_par: bool,
    ) -> Option<Result<&'a CowStr<'input>, Error<usize>>> {
        let exp_prefix = if first_par { "Given " } else { "And " };
        key_paragraph(exp_prefix, " as:", paragraph)
    }

    pub(crate) fn is_when<'a, 'input>(
        paragraph: Tokens<'a, 'input>,
        first_par: bool,
    ) -> Option<Result<&'a CowStr<'input>, Error<usize>>> {
        let exp_prefix = if first_par { "When " } else { "And " };
        key_paragraph(exp_prefix, " is:", paragraph)
    }

    pub(crate) fn is_then<'a, 'input>(
        paragraph: Tokens<'a, 'input>,
        first_par: bool,
    ) -> Option<Result<&'a CowStr<'input>, Error<usize>>> {
        let exp_prefix = if first_par { "Then " } else { "And " };
        key_paragraph(exp_prefix, " is:", paragraph)
    }

    fn key_paragraph<'a, 'input>(
        exp_prefix: &str,
        exp_suffix: &str,
        paragraph: Tokens<'a, 'input>,
    ) -> Option<Result<&'a CowStr<'input>, Error<usize>>> {
        if !util::starts_with(paragraph, exp_prefix) || !util::ends_with(paragraph, exp_suffix) {
            // crate::debug("skip:0", &*paragraph);
            return None; // Ignore paragraphs that don't start or end as expected.
        }
        let [_prefix, key, _suffix] = paragraph else {
            let pattern = format!("{exp_prefix}`<key>`{exp_suffix}");
            let pos = span(&paragraph[0]).start;
            return Some(Err(Error::ExpectedSpecParagraph { pattern, pos }));
        };
        let Event::Code(key) = event(key) else {
            let pattern = format!("{exp_prefix}`<key>`{exp_suffix}");
            let pos = span(key).start;
            return Some(Err(Error::ExpectedSpecParagraph { pattern, pos }));
        };

        Some(Ok(key))
    }
}

// Errors and helper macros
// ========================

/// An error with a generic type `P` that represents the position.
#[derive(Error, Debug, Eq, PartialEq)]
pub enum Error<P: Display> {
    #[error("expected '{pattern}' spec paragraph at {pos}")]
    ExpectedSpecParagraph { pattern: String, pos: P },
    #[error("expected code block after spec paragraph starting at {pos}")]
    ExpectedCode { pos: P },
    #[error("background section at {pos} needs at least one 'Given' paragraph")]
    MissingGiven { pos: P },
    #[error("example section at {pos} needs at least one 'When' paragraph")]
    MissingWhen { pos: P },
    #[error("example section at {pos} needs at least one 'Then' paragraph")]
    MissingThen { pos: P },
}

impl Error<usize> {
    pub fn map_span(self, input: &str) -> Error<Pos> {
        use Error::*;
        let pos_of = |offset: usize| Pos::from(offset, input);
        match self {
            ExpectedSpecParagraph {
                pattern,
                pos: offset,
            } => ExpectedSpecParagraph {
                pattern,
                pos: pos_of(offset),
            },
            ExpectedCode { pos: offset } => ExpectedCode {
                pos: pos_of(offset),
            },
            MissingGiven { pos: offset } => MissingGiven {
                pos: pos_of(offset),
            },
            MissingWhen { pos: offset } => MissingWhen {
                pos: pos_of(offset),
            },
            MissingThen { pos: offset } => MissingThen {
                pos: pos_of(offset),
            },
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Pos {
    line: usize,
    column: usize,
}

impl Pos {
    fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    fn from(mut offset: usize, input: &str) -> Pos {
        let mut rest = input;

        let mut line = 0;
        let mut column = 0;

        while let Some(line_length) = rest.find('\n') {
            if offset < line_length {
                column = offset;
                break;
            } else {
                offset -= line_length + 1;
                line += 1;
                rest = &rest[line_length + 1..];
            }
        }

        Pos::new(line + 1, column + 1)
    }
}

impl Display for Pos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { line, column } = self;
        write!(f, "line {line}, column {column}")
    }
}

#[cfg(test)]
mod tests {
    use indoc;

    use super::super::examples::*;
    use super::{sections, Error, Pos, Section};
    use crate::md;

    #[test]
    fn test_sections() {
        let md_source = make_spec(INPUT_SQL, OUTPUT_SQL);
        let mut md_doc = md::MdDocument::from_string(&md_source);

        // println!("----");
        for section in sections(&mut md_doc) {
            match section {
                Ok(Section::Background(background)) => {
                    // println!("{background:#?}");
                    assert_eq!(background.given.len(), 2);
                    assert!(background.given.contains_key("pipeline"));
                    assert!(background.given.contains_key("environment"));
                }
                Ok(Section::Example(example)) => {
                    // println!("{example:#?}");
                    assert_eq!(example.when.len(), 1);
                    assert!(example.when.contains_key("input"));

                    assert_eq!(example.then.len(), 1);
                    assert!(example.then.contains_key("output"));
                }
                Ok(Section::Raw(_raw)) => {
                    // println!("{raw:#?}");
                    // todo
                }
                Err(err) => {
                    let err = err.map_span(&md_source);
                    panic!("err: {err}");
                }
            }
            // println!("----");
        }
    }

    #[test]
    fn bad_sections() {
        struct TestCase {
            md_source: &'static str,
            exp_error: Error<Pos>,
        }
        let test_cases = [
            TestCase {
                md_source: indoc::indoc! {r"
                    ## Background (1)

                    Given `pipeline` as:
                "},
                exp_error: Error::ExpectedCode {
                    pos: Pos::new(3, 1),
                },
            },
            TestCase {
                md_source: indoc::indoc! {r"
                    ## Example: (1)

                    When pipeline is:
                "},
                exp_error: Error::ExpectedSpecParagraph {
                    pattern: String::from("When `<key>` is:"),
                    pos: Pos::new(3, 1),
                },
            },
            TestCase {
                md_source: indoc::indoc! {r"
                    ## Example: (2)

                    When _pipeline_ is:
                "},
                exp_error: Error::ExpectedSpecParagraph {
                    pattern: String::from("When `<key>` is:"),
                    pos: Pos::new(3, 1),
                },
            },
            TestCase {
                md_source: indoc::indoc! {r"
                    ## Example: (3)

                    When `input` is:

                    ```
                    5
                    ```
                "},
                exp_error: Error::MissingThen {
                    pos: Pos::new(1, 1),
                },
            },
        ];

        for test_case in test_cases {
            let TestCase {
                md_source,
                exp_error,
            } = test_case;
            let mut md_doc = md::MdDocument::from_string(md_source);

            // println!("----");
            for section in sections(&mut md_doc) {
                let act_error = section.expect_err("example errors");
                assert_eq!(exp_error, act_error.map_span(md_source));
                // println!("----");
            }
        }
    }
}
