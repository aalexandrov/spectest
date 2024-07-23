//! Utilities for writing [`MdDocument`] documents.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use fs2::FileExt;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Tag, TagEnd};
use thiserror::Error;

use super::MdDocument;

// Errors and helper macros
// ========================

#[derive(Error, Debug)]
pub enum Error {
    #[error("io error")]
    IO(#[from] std::io::Error),
    #[error("unsupported pulldown_cmark event type {0}")]
    UnsupportedEvent(&'static str),
    #[error("unsupported pulldown_cmark start tag {0}")]
    UnsupportedTag(&'static str),
    #[error("unknown MdWriter error")]
    Unknown,
}

macro_rules! unsupported_event {
    // `()` indicates that the macro takes no argument.
    ($event_type:literal) => {
        // The macro will expand into the contents of this block.
        return Err(Error::UnsupportedEvent($event_type));
    };
}

macro_rules! unsupported_tag {
    // `()` indicates that the macro takes no argument.
    ($tag_type:literal) => {
        // The macro will expand into the contents of this block.
        return Err(Error::UnsupportedTag($tag_type));
    };
}

// MarkdownOutput implementation
// =============================

impl<'input> MdDocument<'input> {
    /// Consume an [`MdDocument`] and write it back into a [`String`].
    pub fn write_to_string(self) -> Result<String, Error> {
        let mut md_writer = MdWriter::new(Vec::new());
        md_writer.write(self)?;
        let string = String::from_utf8(md_writer.out.write);
        Ok(string.expect("valid utf8 string in output buffer"))
    }

    /// Consume an [`MdDocument`] and write it back into the given `path`.
    pub fn write_to_path<P>(self, path: P) -> Result<(), Error>
    where
        P: AsRef<Path>,
    {
        let mut md_writer = MdWriter::new(Vec::new());
        md_writer.write(self)?;

        // Explicitly open with `OpenOptions` in order to avoid truncating the
        // file before obtaining the lock.
        let mut file = OpenOptions::new().write(true).open(&path)?;
        file.lock_exclusive()?;
        file.set_len(0)?;
        file.write_all(md_writer.out.write.as_ref())?;

        Ok(())
    }
}

pub struct MdWriter<W> {
    /// Output writer.
    out: Out<W>,
}

impl<W> MdWriter<W> {
    fn new(write: W) -> Self {
        Self {
            out: Out { write, bytes: 0 },
        }
    }

    fn write(&mut self, input: MdDocument<'_>) -> Result<(), Error>
    where
        W: Write,
    {
        for (event, _span) in input.tokens {
            self.write_event(event)?;
        }
        Ok(())
    }

    fn write_event(&mut self, event: Event<'_>) -> Result<(), Error>
    where
        W: Write,
    {
        match event {
            Event::Start(tag) => {
                self.start(tag)?;
            }
            Event::End(tag) => {
                self.end(tag)?;
            }
            Event::Text(str) => {
                self.out.write_all(str.as_bytes())?;
            }
            Event::Code(str) => {
                self.out.write_all("`".as_ref())?;
                self.out.write_all(str.as_bytes())?;
                self.out.write_all("`".as_ref())?;
            }
            Event::InlineMath(_) => {
                unsupported_event!("InlineMath");
            }
            Event::DisplayMath(_) => {
                unsupported_event!("DisplayMath");
            }
            Event::Html(str) => {
                self.out.write_all(str.as_bytes())?;
            }
            Event::InlineHtml(str) => {
                self.out.write_all(str.as_bytes())?;
            }
            Event::FootnoteReference(_) => {
                unsupported_event!("FootnoteReference");
            }
            Event::SoftBreak => {
                self.out.write_all("\n".as_ref())?;
            }
            Event::HardBreak => {
                self.out.write_all("\n\n".as_ref())?;
            }
            Event::Rule => {
                self.out.write_all("---".as_ref())?;
            }
            Event::TaskListMarker(_) => {
                unsupported_event!("TaskListMarker");
            }
        }
        Ok(())
    }

    pub fn start(&mut self, tag: Tag<'_>) -> Result<(), Error>
    where
        W: Write,
    {
        match tag {
            Tag::Paragraph => {
                self.out.write_separator()?;
            }
            Tag::Heading { level, .. } => {
                self.out.write_separator()?;
                self.out.write_all(Self::heading(level).as_ref())?;
            }
            Tag::BlockQuote(_) => {
                unsupported_tag!("BlockQuote");
            }
            Tag::CodeBlock(CodeBlockKind::Indented) => {
                unsupported_tag!("CodeBlock(CodeBlockKind::Indented)");
            }
            Tag::CodeBlock(CodeBlockKind::Fenced(html)) => {
                self.out.write_separator()?;
                self.out.write_all("```".as_ref())?;
                self.out.write_all(html.as_bytes())?;
                self.out.write_all("\n".as_ref())?;
            }
            Tag::HtmlBlock => {
                self.out.write_separator()?;
            }
            Tag::List(_) => {
                unsupported_tag!("List");
            }
            Tag::Item => {
                unsupported_tag!("Item");
            }
            Tag::FootnoteDefinition(_) => {
                unsupported_tag!("FootnoteDefinition");
            }
            Tag::Table(_) => {
                unsupported_tag!("Table");
            }
            Tag::TableHead => {
                unsupported_tag!("TableHead");
            }
            Tag::TableRow => {
                unsupported_tag!("TableRow");
            }
            Tag::TableCell => {
                unsupported_tag!("TableCell");
            }
            Tag::Emphasis => {
                self.out.write_all("_".as_ref())?;
            }
            Tag::Strong => {
                self.out.write_all("**".as_ref())?;
            }
            Tag::Strikethrough => {
                self.out.write_all("~".as_ref())?;
            }
            Tag::Link { .. } => {
                unsupported_tag!("Link");
            }
            Tag::Image { .. } => {
                unsupported_tag!("Image");
            }
            Tag::MetadataBlock(_) => {
                unsupported_tag!("MetadataBlock");
            }
        }
        Ok(())
    }

    pub fn end(&mut self, tag: TagEnd) -> Result<(), Error>
    where
        W: Write,
    {
        match tag {
            TagEnd::Paragraph => {
                self.out.write_all("\n".as_ref())?;
            }
            TagEnd::Heading(_) => {
                self.out.write_all("\n".as_ref())?;
            }
            TagEnd::BlockQuote => {
                unsupported_tag!("BlockQuote");
            }
            TagEnd::CodeBlock => {
                self.out.write_all("```\n".as_ref())?;
            }
            TagEnd::HtmlBlock => {
                // Do nothing.
            }
            TagEnd::List(_) => {
                unsupported_tag!("List");
            }
            TagEnd::Item => {
                unsupported_tag!("Item");
            }

            TagEnd::FootnoteDefinition => {
                unsupported_tag!("FootnoteDefinition");
            }
            TagEnd::Table => {
                unsupported_tag!("Table");
            }
            TagEnd::TableHead => {
                unsupported_tag!("TableHead");
            }
            TagEnd::TableRow => {
                unsupported_tag!("TableRow");
            }
            TagEnd::TableCell => {
                unsupported_tag!("TableCell");
            }
            TagEnd::Emphasis => {
                self.out.write_all("_".as_ref())?;
            }
            TagEnd::Strong => {
                self.out.write_all("**".as_ref())?;
            }
            TagEnd::Strikethrough => {
                self.out.write_all("~".as_ref())?;
            }
            TagEnd::Link => {
                unsupported_tag!("Link");
            }
            TagEnd::Image => {
                unsupported_tag!("Image");
            }
            TagEnd::MetadataBlock(_) => {
                unsupported_tag!("MetadataBlock");
            }
        }
        Ok(())
    }

    fn heading(level: HeadingLevel) -> &'static str
    where
        W: Write,
    {
        match level {
            HeadingLevel::H1 => "# ",
            HeadingLevel::H2 => "## ",
            HeadingLevel::H3 => "### ",
            HeadingLevel::H4 => "#### ",
            HeadingLevel::H5 => "##### ",
            HeadingLevel::H6 => "###### ",
        }
    }
}

// Helper structs
// ==============

struct Out<W> {
    write: W,
    bytes: usize,
}

impl<W> Out<W> {
    fn write_separator(&mut self) -> std::io::Result<()>
    where
        W: Write,
    {
        if self.bytes > 0 {
            self.write_all("\n".as_ref())?;
        }
        Ok(())
    }
}

impl<W: Write> Write for Out<W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let bytes = self.write.write(buf)?;
        self.bytes += bytes;
        Ok(bytes)
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.write.write_all(buf)?;
        self.bytes += buf.len();
        Ok(())
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        self.write.flush()
    }
}
