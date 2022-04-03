use proc_macro::TokenStream;
use quote::quote;
use std::{fs, path::PathBuf};
use syn::{
    parse::{Parse, ParseStream, Result},
    parse_macro_input, Ident,
};

struct MacroAttributes {
    input_regex: String,
    output_template: Option<String>,
}

impl Parse for MacroAttributes {
    fn parse(input: ParseStream) -> Result<Self> {
        let input_ident: syn::Ident = input.parse()?;
        assert_eq!(input_ident.to_string(), "input");
        let _equals: syn::token::Eq = input.parse()?;
        let input_regex = input.parse::<syn::LitStr>()?.value();
        if let Ok(_comma) = input.parse::<syn::token::Comma>() {
            let output_ident: syn::Ident = input.parse()?;
            assert_eq!(output_ident.to_string(), "output");
            let _equals: syn::token::Eq = input.parse()?;
            let output_template = input.parse::<syn::LitStr>()?.value();
            Ok(Self {
                input_regex,
                output_template: Some(output_template),
            })
        } else {
            Ok(Self {
                input_regex,
                output_template: None,
            })
        }
    }
}

struct TestCase {
    test_name: String,
    input_file: String,
    output_file: Option<String>,
}

impl TestCase {
    fn into_test_fn_tokens(
        self,
        impl_ident: &Ident,
        remove_crlf: bool,
    ) -> proc_macro2::TokenStream {
        let fn_ident = quote::format_ident!("{}", self.test_name);
        let input_file = self.input_file;
        if let Some(output_file) = self.output_file {
            // `output` file was given,
            // so we're testing that the function return matches the contents
            // of the `output` file if it exists.
            //
            // If it doesn't exist then we write it.
            quote! {
                #[test]
                fn #fn_ident() {
                    let mut input_contents = std::fs::read_to_string(#input_file).unwrap();
                    if (#remove_crlf) {
                        input_contents = input_contents.replace("\r\n", "\n");
                    }
                    let want = #impl_ident(&input_contents);
                    let got_path = std::path::PathBuf::from(#output_file);
                    if got_path.exists() {
                        let mut got = std::fs::read_to_string(got_path).unwrap();
                        if (#remove_crlf) {
                            got = got.replace("\r\n", "\n");
                        }
                        similar_asserts::assert_str_eq!(got: got, want: want);
                    } else {
                        std::fs::write(got_path, want).unwrap();
                    }
                }
            }
        } else {
            // No `output` file,
            // so we're testing that the function return matches the `input`
            quote! {
                #[test]
                fn #fn_ident() {
                    let mut input_contents = std::fs::read_to_string(#input_file).unwrap();
                    if (#remove_crlf) {
                        input_contents = input_contents.replace("\r\n", "\n");
                    }
                    let want = #impl_ident(&input_contents);
                    let got = input_contents;
                    similar_asserts::assert_str_eq!(got: got, want: want);
                }
            }
        }
    }
}

#[proc_macro_attribute]
pub fn snapshot_lf(attrs: TokenStream, func: TokenStream) -> TokenStream {
    snapshot_impl(attrs, func, true)
}

#[proc_macro_attribute]
pub fn snapshot(attrs: TokenStream, func: TokenStream) -> TokenStream {
    snapshot_impl(attrs, func, false)
}

fn snapshot_impl(attrs: TokenStream, func: TokenStream, remove_crlf: bool) -> TokenStream {
    let attrs = parse_macro_input!(attrs as MacroAttributes);

    let mut snapshot_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    snapshot_dir.push(PathBuf::from(&attrs.input_regex).parent().unwrap());

    let input_file_name_regex = regex::Regex::new(
        PathBuf::from(attrs.input_regex)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap(),
    )
    .unwrap();

    let mut test_cases: Vec<TestCase> = Vec::new();
    for entry in fs::read_dir(&snapshot_dir).unwrap() {
        let entry = entry.unwrap();
        let entry_path = entry.path();
        let file_name = entry_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        if !input_file_name_regex.is_match(&file_name) {
            continue;
        }
        let test_name = entry_path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        let input_file = entry_path.to_string_lossy().into_owned();
        let output_file = attrs.output_template.clone().map(|template| {
            if let Some(captures) = input_file_name_regex.captures(&file_name) {
                if let Some(capture) = captures.get(1) {
                    template.replace("${1}", capture.as_str())
                } else {
                    template
                }
            } else {
                template
            }
        });

        test_cases.push(TestCase {
            test_name,
            input_file,
            output_file,
        });
    }

    let func_ast: syn::ItemFn = syn::parse(func).expect("failed to parse tokens as a function");
    let func_ident = func_ast.sig.ident.clone();

    let test_cases =
        test_cases
            .into_iter()
            .fold(proc_macro2::TokenStream::new(), |mut tokens, test_case| {
                tokens.extend(test_case.into_test_fn_tokens(&func_ident, remove_crlf));
                tokens
            });
    quote! {
        #[cfg(test)]
        mod #func_ident {
            // TODO: add an include_dir! call here to create a data dependency
            // https://internals.rust-lang.org/t/pre-rfc-add-a-builtin-macro-to-indicate-build-dependency-to-file/9242
            use super::*;

            #test_cases

            #func_ast
        }
    }
    .into()
}
