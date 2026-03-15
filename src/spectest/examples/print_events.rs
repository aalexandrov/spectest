//! Self-contained example: parse a markdown file with pulldown_cmark and print
//! the list of events (with byte ranges).
//!
//! Usage:
//!   cargo run -p spectest --example print_events -- path/to/file.md
//!   cargo run -p spectest --example print_events -- path/to/dir/*.md

use std::env;
use std::fs;
use std::path::Path;

use pulldown_cmark::{Event, Options, Parser};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: print_events <path-to-markdown-file> [path2 ...]");
        eprintln!("Example: print_events doc/README.md");
        std::process::exit(1);
    }

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);

    for path_str in args.iter().skip(1) {
        let path = Path::new(path_str);
        if !path.exists() {
            eprintln!("No such file or directory: {}", path.display());
            continue;
        }
        if path.is_dir() {
            eprintln!("Skipping directory (pass files): {}", path.display());
            continue;
        }

        let source = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to read {}: {}", path.display(), e);
                continue;
            }
        };

        println!("{}\n", path.display());
        println!("   i   | min....max | event");
        println!("------:|:----------:|:---------------------------------");
        let parser = Parser::new_ext(&source, options);
        for (i, (event, range)) in parser.into_offset_iter().enumerate() {
            print_event(i, &range, &event);
        }
        println!("```\n");
    }
}

fn print_event(i: usize, r: &std::ops::Range<usize>, event: &Event<'_>) {
    println!("{: >4}.  | {:.<4}..{:.>4} | {:?}", i, r.start, r.end, event);
}
