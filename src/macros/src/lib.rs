//! Macros for the [`spectest`](../spectest/index.html) package.

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::spanned::Spanned;
use syn::{self};

/// A macro that expands a single test case parameterized by a single `&str`
/// parameter into a family of tests with the same name used as prefix for each
/// file found in a [`glob`](../glob/index.html) pattern at compile time.
///
/// # Example
///
/// Assume that your cargo project has a `testdata` folder with the following
/// structure:
///
/// - `foo`
///    - `bar.md`
///    - `baz.md`
///
/// Then the expansion
///
/// ```
/// use spectest_macros::glob_test;
///
/// #[glob_test("testdata/foo/**/*.md")]
/// fn test_foo(path: &str) {
///     println!("Running test at path = {path}");
/// }
/// ```
///
/// will be
///
/// ```
/// fn test_foo(path: &str) {
///     println!("Running test at path = {path}");
/// }
///
/// #[test]
/// fn test_foo_bar() {
///     test_foo("/path/to/crate/testdata/foo/bar.md")
/// }
///
/// #[test]
/// fn test_foo_baz() {
///     test_foo("/path/to/crate/testdata/foo/baz.md")
/// }
/// ```
#[proc_macro_attribute]
pub fn glob_test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let Ok(syn::Lit::Str(glob_pattern)) = syn::parse(attr) else {
        let msg = "glob_test: needs a glob pattern literal string parameter";
        let err = syn::Error::new(Span::call_site(), msg);
        return err.to_compile_error().into();
    };

    // This seems to be
    let glob_resolved = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(path) => {
            let glob_pattern = glob_pattern.value().to_string();
            format!("{path}/{glob_pattern}") // TODO: find a safer way to do this
        }
        Err(_) => glob_pattern.value().to_string(),
    };

    let Ok(syn::ItemFn {
        attrs,
        vis,
        sig,
        block,
    }) = syn::parse(item)
    else {
        let msg = "glob_test: attribute can only annotate a function";
        let err = syn::Error::new(Span::call_site(), msg);
        return err.to_compile_error().into();
    };

    if let Err(err) = check_signature(&sig) {
        return err;
    };

    let Ok(paths) = glob::glob(&glob_resolved) else {
        let msg = "glob_test: argument is not a valid glob pattern";
        let err = syn::Error::new(glob_pattern.span(), msg);
        return err.to_compile_error().into();
    };

    let const_prefix_len = glob_resolved.find("*").unwrap_or(0);
    let test_attrs = std::iter::repeat(attrs.clone());
    let fn_name = &sig.ident;
    let mut test_sig = Vec::new();
    let mut test_block = Vec::new();
    for entry in paths {
        match entry {
            Ok(path) => {
                if path.to_str().is_none() {
                    let msg = "glob_test: pattern contains a non-utf8 path";
                    let err = syn::Error::new(glob_pattern.span(), msg);
                    return err.to_compile_error().into();
                }

                test_sig.push({
                    let test_signature = syn::Signature {
                        ident: {
                            let prefix = sig.ident.to_string();
                            let suffix = path
                                .with_extension("")
                                .to_string_lossy() // lossless conversion asserted above
                                .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
                                .split_off(const_prefix_len);
                            let test_fn_name = format!("{}_{}", &prefix, &suffix);
                            syn::Ident::new(&test_fn_name, sig.ident.span())
                        },
                        inputs: syn::punctuated::Punctuated::new(),
                        ..sig.clone()
                    };
                    Box::new(test_signature)
                });

                let path = path.to_str();

                test_block.push({
                    let value = syn::parse2::<syn::Block>(quote::quote! {
                        {
                            #fn_name(#path)
                        }
                    });
                    Box::new(value.expect("test body"))
                });
            }
            Err(err) => {
                let err = syn::Error::new(glob_pattern.span(), err);
                return err.to_compile_error().into();
            }
        };
    }

    if test_sig.is_empty() {
        let msg = format!("glob_test: resolved pattern `{glob_resolved}` didn't match any paths");
        let err = syn::Error::new(glob_pattern.span(), msg);
        return err.to_compile_error().into();
    }

    // let mut key = vec![];
    // let mut val = vec![];
    // for (k, v) in std::env::vars() {
    //     key.push(syn::Lit::Str(syn::LitStr::new(&k, glob_pattern.span())));
    //     val.push(syn::Lit::Str(syn::LitStr::new(&v, glob_pattern.span())));
    // }

    // Replace the original parameterized test with specialized tests for each
    // string path matching the glob pattern.
    let expanded = quote! {
        #(#attrs)* #vis #sig #block

        // #[test] fn test_current_env() {
        //     #(
        //         print!("{}", #key);
        //         print!(": ");
        //         print!("{}", #val);
        //         println!();
        //     )*
        // }

        #( #(#test_attrs)* #[test] #vis #test_sig #test_block )*
    };

    // Convert into a token stream and return it
    expanded.into()
}

fn check_signature(sig: &syn::Signature) -> Result<&Ident, TokenStream> {
    if sig.inputs.len() != 1 {
        let span = if sig.inputs.is_empty() {
            sig.ident.span()
        } else {
            sig.inputs.iter().skip(1).next().unwrap().span()
        };
        let msg = "glob_test: annotated function must have exactly one parameter";
        let err = syn::Error::new(span, msg);
        return Err(err.to_compile_error().into());
    }

    let fn_arg = sig.inputs.last().expect("fn arg");

    match &fn_arg {
        syn::FnArg::Typed(syn::PatType {
            attrs,
            pat,
            colon_token: _,
            ty,
        }) => {
            if !attrs.is_empty() {
                let msg = "glob_test: function parameter cannot have attributes";
                let err = syn::Error::new(fn_arg.span(), msg);
                return Err(err.to_compile_error().into());
            }

            if !is_str(ty) {
                let msg = "glob_test: function parameter type must be `&str`";
                let err = syn::Error::new(fn_arg.span(), msg);
                return Err(err.to_compile_error().into());
            }

            match pat.as_ref() {
                syn::Pat::Ident(syn::PatIdent {
                    attrs,
                    by_ref: None,
                    mutability: None,
                    ident,
                    subpat: None,
                }) if attrs.is_empty() => Ok(ident.into()),
                _ => {
                    let msg = "glob_test: function parameter must bind a variable";
                    let err = syn::Error::new(fn_arg.span(), msg);
                    return Err(err.to_compile_error().into());
                }
            }
        }
        syn::FnArg::Receiver(_) => {
            let msg = "glob_test: function parameter must not be `self`";
            let err = syn::Error::new(fn_arg.span(), msg);
            return Err(err.to_compile_error().into());
        }
    }
}

fn is_str(path: &syn::Type) -> bool {
    match path {
        syn::Type::Reference(syn::TypeReference {
            and_token: _,
            lifetime: None,
            mutability: None,
            elem,
        }) => match elem.as_ref() {
            syn::Type::Path(syn::TypePath {
                qself: None,
                path:
                    syn::Path {
                        leading_colon: None,
                        segments,
                    },
            }) if segments.len() == 1 => match segments.last() {
                Some(syn::PathSegment {
                    ident,
                    arguments: syn::PathArguments::None,
                }) => ident == "str",
                _ => false,
            },
            _ => false,
        },
        _ => false,
    }
}
