use std::io::Write;

use pulldown_cmark::{Event, HeadingLevel, Tag, TagEnd};
use thiserror::Error;

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

pub struct MdWriter<'a, W> {
    /// Output writer.
    out: Out<W>,
    /// Open tags that need to be remembered for proper closing.
    open_tags: Vec<Tag<'a>>,
}

impl<'a, W> MdWriter<'a, W> {
    pub fn new(write: W) -> Self {
        Self {
            out: Out::new(write),
            open_tags: Vec::new(),
        }
    }

    pub fn into_write(self) -> W {
        self.out.write
    }

    pub fn into_string(self) -> String
    where
        W: Into<Vec<u8>>,
    {
        let utf8 = self.out.write.into();
        String::from_utf8(utf8).expect("valid utf8 string in output buffer")
    }

    pub fn write(&mut self, event: Event<'a>) -> Result<(), Error>
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
                self.out.write_all(str.as_bytes())?;
            }
            Event::InlineMath(_) => {
                unsupported_event!("InlineMath");
            }
            Event::DisplayMath(_) => {
                unsupported_event!("DisplayMath");
            }
            Event::Html(_) => {
                unsupported_event!("Html");
            }
            Event::InlineHtml(_) => {
                unsupported_event!("InlineHtml");
            }
            Event::FootnoteReference(_) => {
                unsupported_event!("FootnoteReference");
            }
            Event::SoftBreak => {
                self.out.write_all("\n".as_ref())?;
                self.prefix()?;
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

    pub fn start(&mut self, tag: Tag<'a>) -> Result<(), Error>
    where
        W: Write,
    {
        match tag {
            Tag::Paragraph => {
                if self.out.bytes > 0 {
                    self.prefix()?;
                    self.out.write_all("\n".as_ref())?;
                }
                self.prefix()?;
            }
            Tag::Heading { level, .. } => {
                if self.out.bytes > 0 {
                    self.out.write_all("\n".as_ref())?;
                }
                self.out.write_all(Self::heading(level).as_ref())?;
            }
            Tag::BlockQuote(_) => {
                if self.out.bytes > 0 {
                    self.out.write_separator()?;
                }
                self.open_tags.push(tag);
            }
            Tag::CodeBlock(_) => {
                self.out.write_all("```".as_ref())?;
            }
            Tag::HtmlBlock => {
                unsupported_tag!("HtmlBlock");
            }
            Tag::List(_) => {
                if self.out.bytes > 0 && self.count(|tag| matches!(tag, Tag::List(_))) == 0 {
                    self.out.write_separator()?;
                }
                self.open_tags.push(tag);
            }
            Tag::Item => {
                let level = self.count(|tag| matches!(tag, Tag::List(_)));
                let Some(Tag::List(n)) = self.find(|tag| matches!(tag, Tag::List(_))) else {
                    // TODO: return an Error instead.
                    unreachable!("expected at least one Tag::List in open_tags");
                };
                if let Some(n) = n.as_mut() {
                    *n += 1;
                }
                let n = n.clone();
                if level > 1 {
                    self.out.write_all("\n".as_ref())?;
                    for _ in 1..level {
                        self.out.write_all("  ".as_ref())?;
                    }
                }
                match n.clone() {
                    Some(n) => write!(self.out, "{}. ", n - 1)?,
                    None => write!(self.out, "- ")?,
                }
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
                let tag = self.open_tags.pop().expect("BlockQuote tag");
                assert!(matches!(tag, Tag::BlockQuote(_)));
            }
            TagEnd::CodeBlock => {
                self.out.write_all("```".as_ref())?;
            }
            TagEnd::HtmlBlock => {
                unsupported_tag!("HtmlBlock");
            }
            TagEnd::List(_) => {
                let tag = self.open_tags.pop().expect("List tag");
                assert!(matches!(tag, Tag::List(_)));
            }
            TagEnd::Item => {
                self.out.write_all("\n".as_ref())?;
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

    fn prefix(&mut self) -> Result<(), Error>
    where
        W: Write,
    {
        for tag in self.open_tags.iter() {
            match tag {
                Tag::BlockQuote(_) => {
                    self.out.write_all("> ".as_ref())?;
                }
                Tag::List(None) => {
                    // Account for the `- ` prefix of the parent list.
                    self.out.write_all("  ".as_ref())?;
                }
                Tag::List(Some(n)) => {
                    // Account for the `n` digits of the parent list item.
                    for _digit in 0..(n.checked_ilog10().unwrap_or(0) + 1) {
                        self.out.write_all(" ".as_ref())?;
                    }
                    // Align with the the `. ` suffix of the parent list item.
                    self.out.write_all("  ".as_ref())?;
                }
                _ => unreachable!(),
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

    fn find<P>(&mut self, p: P) -> Option<&mut Tag<'a>>
    where
        P: FnMut(&&mut Tag<'a>) -> bool,
    {
        self.open_tags.iter_mut().rev().find(p)
    }

    fn count<P>(&mut self, p: P) -> usize
    where
        P: FnMut(&&Tag<'a>) -> bool,
    {
        self.open_tags.iter().filter(p).count()
    }
}

// Helper structs
// ==============

struct Out<W> {
    write: W,
    bytes: usize,
}

impl<W> Out<W> {
    fn new(write: W) -> Self {
        Self { write, bytes: 0 }
    }

    fn write_separator(&mut self) -> std::io::Result<()>
    where
        W: Write,
    {
        self.bytes = 0;
        self.write.write_all("\n".as_ref())
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
