use pulldown_cmark::{Options, Parser};

mod md_writer;

pub fn md_reader<'input>(input: &'input str) -> Parser<'input> {
    // Set up options and parser.
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    Parser::new_ext(&input, options)
}

pub fn md_writer<'a, W>(write: W) -> md_writer::MdWriter<'a, W> {
    md_writer::MdWriter::new(write)
}
