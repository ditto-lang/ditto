#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use ditto_ast::{self as ast, ModuleExports, ModuleName};
use html_builder::Html5;
use std::fmt::Write;

/// Generate HTML documentation for a module.
pub fn generate_html_docs(module_name: ModuleName, module_exports: ModuleExports) -> String {
    if let Ok(html) = try_generate_html_docs(module_name, module_exports) {
        html
    } else {
        unreachable!() // i hope
    }
}

// TODO: generate index.html

fn try_generate_html_docs(
    module_name: ModuleName,
    module_exports: ModuleExports,
) -> Result<String, std::fmt::Error> {
    let mut buf = html_builder::Buffer::new();
    buf.doctype();

    let mut html = buf.html().attr("lang='en'");

    let mut head = html.head();
    writeln!(head.title(), "📜 {}", module_name)?;
    head.link()
        .attr("rel='stylesheet'")
        .attr("href='./ditto.css'");
    head.link()
        .attr("rel='stylesheet'")
        .attr("href='./styles.css'");
    head.link().attr("rel='stylesheet'").attr(
        "href='https://fonts.googleapis.com/css2?family=Fira+Code:wght@300;400&display=swap'",
    );

    let mut body = html.body();
    writeln!(body.h1(), "{}", module_name)?;

    let ModuleExports {
        types,
        constructors: _, // TODO: use this to find type constructors
        values,
    } = module_exports;
    for module_export in sort_module_exports(types, values) {
        match module_export {
            ModuleExport::Type(type_name, exported_type) => {
                let mut section = body.section().attr(&format!("id='{}'", type_name));
                let doc_comments = exported_type.doc_comments();
                if !doc_comments.is_empty() {
                    writeln!(section, "{}", doc_comments_to_html(doc_comments.clone()))?;
                }
                write!(
                    section,
                    // Inlining the HTML because I don't want html_builder to add whitespace
                    "<pre><code>type {}</code></pre>",
                    type_name
                )?;
                // TODO: handle type variables
            }
            ModuleExport::Value(name, exported_value) => {
                let mut section = body.section().attr(&format!("id='{}'", name));
                if !exported_value.doc_comments.is_empty() {
                    writeln!(
                        section,
                        "{}",
                        doc_comments_to_html(exported_value.doc_comments)
                    )?;
                }
                write!(
                    section,
                    // Inlining the HTML because I don't want html_builder to add whitespace
                    "<pre><code>{} : {}</code></pre>",
                    name,
                    html_escape::encode_text(&exported_value.value_type.debug_render())
                )?;
            }
        }
    }

    Ok(buf.finish())
}

enum ModuleExport {
    Type(ast::ProperName, ast::ModuleExportsType),
    Value(ast::Name, ast::ModuleExportsValue),
}

fn sort_module_exports(
    types: ast::ModuleExportsTypes,
    values: ast::ModuleExportsValues,
) -> Vec<ModuleExport> {
    let mut exports: Vec<(usize, ModuleExport)> = types
        .into_iter()
        .map(|(type_name, exported_type)| {
            (
                exported_type.doc_position(),
                ModuleExport::Type(type_name, exported_type),
            )
        })
        .chain(values.into_iter().map(|(name, exported_value)| {
            (
                exported_value.doc_position,
                ModuleExport::Value(name, exported_value),
            )
        }))
        .collect();
    exports.sort_by_key(|(doc_position, _)| *doc_position);
    exports.into_iter().map(|(_, export)| export).collect()
}

fn doc_comments_to_html(doc_comments: Vec<String>) -> String {
    // TODO: syntax highlight ditto code blocks
    markdown_to_html(&unindent::unindent(&doc_comments.join("\n")))
}

fn markdown_to_html(md: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(md, options);

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

#[cfg(test)]
mod tests {
    #[snapshot_test::snapshot_lf(
        input = "golden-tests/(.*).ditto",
        output = "golden-tests//${1}.html"
    )]
    fn golden(input: &str) -> String {
        let cst_module = ditto_cst::Module::parse(input).unwrap();
        let (ast_module, _warnings) =
            ditto_checker::check_module(&ditto_checker::Everything::default(), cst_module).unwrap();
        let ditto_ast::Module {
            module_name,
            exports: module_exports,
            ..
        } = ast_module;
        prettier(&crate::generate_html_docs(module_name, module_exports))
    }
    /// Use prettier to make sure the generated code is valid syntactically.
    /// (and make it look nice)
    fn prettier(text: &str) -> String {
        use std::{
            io::Write,
            process::{Command, Stdio},
        };

        let mut child = Command::new("node")
            // NOTE: node_modules/.bin/prettier is a shell script on windows
            .arg("../../node_modules/prettier/bin-prettier.js")
            .arg("--parser")
            .arg("html")
            // NOTE: prettier defaults to `--end-of-line=lf`
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(text.as_bytes()).unwrap();
        drop(stdin);

        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    }
}
