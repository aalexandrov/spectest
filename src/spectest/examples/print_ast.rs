//! Example: parse a markdown file with markdown-ast (via pulldown_cmark events),
//! convert events to an AST with `events_to_ast`, and print the AST.
//!
//! Usage:
//!   cargo run -p spectest --example print_ast -- path/to/file.md
//!   cargo run -p spectest --example print_ast -- path/to/dir/*.md

use std::env;
use std::fs;
use std::path::Path;

use markdown_ast::{events_to_ast, markdown_to_events};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: print_ast <path-to-markdown-file> [path2 ...]");
        eprintln!("Example: print_ast doc/README.md");
        std::process::exit(1);
    }

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
        let events = markdown_to_events(&source);
        let ast = events_to_ast(events);
        println!("{:#?}", ast);
        println!();
    }
}
