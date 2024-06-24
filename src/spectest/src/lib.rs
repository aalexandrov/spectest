//! A lightweight library for defining behavior-driven development (BDD) style
//! tests in external files and running them with `cargo test`.
//!
//! To write a test:
//!
//! 1. Implement a [`Handler`] that interprets [`Background`] and [`Example`]
//!    sections defined in your spec file.
//! 2. Write a test that calls [`run`] with a `Handler` instance and a path that
//!    points to a spec file. You can also use [`glob_test`] to derive one such
//!    test for each spec file in a given folder (including subfolders).
//!
//! # Example
//!
//! Here is a minimal example:
//!
//! ```
//! use spectest;
//!
//! struct MevalHandler<'a> {
//!     ctx: meval::Context<'a>,
//! }
//!
//! impl<'a> MevalHandler<'a> {
//!     fn new() -> Self {
//!         Self {
//!             ctx: meval::Context::new(),
//!         }
//!     }
//! }
//!
//! impl<'a> spectest::Handler for MevalHandler<'a> {
//!     type Error = String;
//!
//!     fn example(&mut self, example: &mut spectest::Example) -> Result<(), Self::Error> {
//!         let Some(input) = example.when.get("input") else {
//!             let msg = format!("missing `input` definition in the 'When' spec");
//!             return Err(msg);
//!         };
//!         let input = match input.parse::<meval::Expr>() {
//!             Ok(expr) => expr,
//!             Err(err) => {
//!                 let msg = format!("cannot parse `input` expression `{input}`: {err}");
//!                 return Err(msg);
//!             }
//!         };
//!
//!         match input.eval_with_context(self.ctx.clone()) {
//!             Ok(value) => {
//!                 example.then.insert("result", value.to_string() + "\n");
//!             }
//!             Err(err) => {
//!                 let msg = format!("cannot evaluate expression: {err}\n");
//!                 example.then.insert("result", msg);
//!             }
//!         }
//!
//!         Ok(())
//!     }
//! }
//!
//! #[spectest::glob_test("testdata/integration/**/*.md")]
//! fn test_meval(path: &str) {
//!     let mut handler = MevalHandler::new();
//!     spectest::run(path, &mut handler);
//! }
//! ```
//!
//! Assuming that the `testdata/integration` folder contains a single called
//! `calculator.md`, one can run the test against this file as follows:
//!
//! ```bash
//! # Expand the prefix to narrow the set of tested spec files
//! cargo test test_meval_
//! ```
//!
//! It is also possible to mass-rewrite failing tests after fixing/updating the
//! behavior of the `meval` library under test as follows
//! ```bash
//! REWRITE_SPECS=true cargo test test_calculator
//! ```
//!
//! For a more elaborated version that also updates the evaluation context
//! depending on the currently active [`Background`] sections, see the
//! `test/integration.rs` in the source repository.

use std::ops::Range;

use pulldown_cmark::Event;

pub mod core;
pub mod md;

pub use core::{run, Background, Error, Example, Handler};
#[cfg(feature = "macros")]
pub use spectest_macros::glob_test;

// Common private helper types
// ===========================

type Token<'input> = (Event<'input>, Range<usize>);
type Tokens<'a, 'input> = &'a mut [Token<'input>];

/// Project the `event` component a `token`.
#[inline(always = true)]
fn event<'a, 'input>(token: &'a Token<'input>) -> &'a Event<'input> {
    &token.0
}

/// Project the `span` component a `token`.
#[inline(always = true)]
fn span<'a>(token: &'a Token<'_>) -> &'a Range<usize> {
    &token.1
}

/// Print a tokens sequence for debugging purposes.
#[allow(unused)]
pub(crate) fn debug(tag: &str, tokens: &[Token<'_>]) {
    println!("<{tag}>");
    for (i, (event, span)) in tokens.iter().enumerate() {
        println!("{i:03} at: span={span:03?} - event={event:?}");
    }
    println!("</{tag}>");
}
