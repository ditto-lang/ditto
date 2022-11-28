use super::{
    declaration::gen_declaration,
    helpers::space,
    name::{gen_module_name, gen_name, gen_package_name, gen_proper_name},
    syntax::{gen_parens, gen_parens_list1},
    token::{
        gen_as_keyword, gen_close_paren, gen_double_dot, gen_exports_keyword, gen_import_keyword,
        gen_module_keyword, gen_open_paren, gen_semicolon,
    },
};
use ditto_cst::{Everything, Export, Exports, Header, Import, ImportLine, ImportList, Module};
use dprint_core::formatting::{PrintItems, Signal};

pub fn gen_module(module: Module) -> PrintItems {
    let mut items = PrintItems::new();
    items.extend(gen_module_header(module.header));
    items.push_signal(Signal::NewLine);

    if !module.imports.is_empty() {
        items.push_signal(Signal::NewLine);
        let mut sorted_import_lines = module.imports;
        sorted_import_lines.sort_by_key(|import_line| {
            let package_name = import_line
                .package
                .as_ref()
                .map(|parens| parens.value.0.value.clone());
            let mut module_name = import_line
                .module_name
                .init
                .iter()
                .map(|(proper_name, _dot)| proper_name.0.value.clone())
                .collect::<Vec<_>>();
            module_name.push(import_line.module_name.last.0.value.clone());

            (
                std::cmp::Reverse(package_name.map(std::cmp::Reverse)),
                module_name,
            )
        });

        let mut previous_package_name = None;
        for (i, import_line) in sorted_import_lines.into_iter().enumerate() {
            let package_name = import_line
                .package
                .as_ref()
                .map(|parens| parens.value.0.value.clone());
            if i != 0 && package_name != previous_package_name {
                items.push_signal(Signal::NewLine);
            }
            items.extend(gen_import_line(import_line));
            items.push_signal(Signal::NewLine);
            previous_package_name = package_name;
        }
    }

    let module_declarations_empty = module.declarations.is_empty();
    let declarations_len = module.declarations.len();
    for declaration in module.declarations {
        items.push_signal(Signal::NewLine);
        items.push_signal(Signal::NewLine);
        items.extend(gen_declaration(declaration));
    }

    if !module.trailing_comments.is_empty() {
        if declarations_len > 0 {
            items.push_signal(Signal::NewLine);
        }
        items.push_signal(Signal::NewLine);
        items.push_signal(Signal::NewLine);
        for comment in module.trailing_comments.iter() {
            items.push_string(comment.0.trim_end().to_string());
            items.push_signal(Signal::NewLine);
        }
    } else if !module_declarations_empty {
        items.push_signal(Signal::NewLine);
    }
    items
}

fn gen_module_header(header: Header) -> PrintItems {
    let mut items = PrintItems::new();
    items.extend(gen_module_keyword(header.module_keyword));
    items.extend(space());
    items.extend(gen_module_name(header.module_name));
    items.extend(space());
    items.extend(gen_exports_keyword(header.exports_keyword));
    items.extend(space());
    items.extend(gen_exports(header.exports));
    items.extend(gen_semicolon(header.semicolon));
    items
}

fn gen_exports(exports: Exports) -> PrintItems {
    match exports {
        Exports::Everything(everything) => gen_everything(everything),
        Exports::List(box list) => gen_parens_list1(list, gen_export, true),
    }
}

fn gen_everything(everything: Everything) -> PrintItems {
    gen_parens(everything, gen_double_dot)
}

fn gen_export(export: Export) -> PrintItems {
    match export {
        Export::Value(name) => gen_name(name),
        Export::Type(proper_name, everything) => {
            let mut items = PrintItems::new();
            items.extend(gen_proper_name(proper_name));
            if let Some(everything) = everything {
                items.extend(gen_everything(everything));
            }
            items
        }
    }
}

fn gen_import_line(import_line: ImportLine) -> PrintItems {
    let mut items = PrintItems::new();
    items.extend(gen_import_keyword(import_line.import_keyword));
    items.extend(space());
    if let Some(package) = import_line.package {
        items.extend(gen_open_paren(package.open_paren));
        items.extend(gen_package_name(package.value));
        items.extend(gen_close_paren(package.close_paren));
        items.extend(space());
    }
    items.extend(gen_module_name(import_line.module_name));
    if let Some((as_keyword, proper_name)) = import_line.alias {
        items.extend(space());
        items.extend(gen_as_keyword(as_keyword));
        items.extend(space());
        items.extend(gen_proper_name(proper_name));
    }
    if let Some(import_list) = import_line.imports {
        items.extend(space());
        items.extend(gen_import_list(import_list));
    }
    items.extend(gen_semicolon(import_line.semicolon));
    items
}

fn gen_import_list(import_list: ImportList) -> PrintItems {
    gen_parens_list1(import_list.0, gen_import, true)
}

fn gen_import(import: Import) -> PrintItems {
    match import {
        Import::Value(name) => gen_name(name),
        Import::Type(proper_name, everything) => {
            let mut items = PrintItems::new();
            items.extend(gen_proper_name(proper_name));
            if let Some(everything) = everything {
                items.extend(gen_everything(everything));
            }
            items
        }
    }
}

#[cfg(test)]
mod tests {
    mod module_header {
        macro_rules! assert_fmt {
            ($source:expr) => {{
                assert_fmt!($source, $source, $crate::config::MAX_WIDTH)
            }};
            ($source:expr, $want:expr) => {{
                assert_fmt!($source, $want, $crate::config::MAX_WIDTH)
            }};
            ($source:expr, $want:expr, $max_width:expr) => {{
                let items =
                    $crate::module::gen_module_header(ditto_cst::Header::parse($source).unwrap());
                $crate::test_macros::assert_fmt!(items, $source, $want, $max_width);
            }};
        }

        #[test]
        fn it_formats_module_headers() {
            assert_fmt!("module Test exports (..);");
            assert_fmt!("module Foo.Bar.Baz exports (..);");
            assert_fmt!("module T exports (foo);", "module T exports (\n\tfoo,\n);");
            assert_fmt!(
                "module T exports (foo,bar,baz);",
                "module T exports (\n\tfoo,\n\tbar,\n\tbaz,\n);"
            );
            assert_fmt!("module T exports (Foo);", "module T exports (\n\tFoo,\n);");
            assert_fmt!(
                "module T exports (Foo,Bar,Baz);",
                "module T exports (\n\tFoo,\n\tBar,\n\tBaz,\n);"
            );
            assert_fmt!(
                "module T exports (Foo,Bar(..),    Baz);",
                "module T exports (\n\tFoo,\n\tBar(..),\n\tBaz,\n);"
            );

            assert_fmt!("module T exports (foo,);", "module T exports (\n\tfoo,\n);");
            assert_fmt!("-- comment\nmodule Test exports (..);");
            assert_fmt!("module  -- comment\n Test exports (..);");
            assert_fmt!("module Test  -- comment\n exports (..);");
            assert_fmt!("module Test exports  -- comment\n (..);");
            assert_fmt!("module  -- comment\n Test exports  -- comment\n (..);");
            assert_fmt!("module A.B.C exports (  -- comment\n\t..\n);");
            assert_fmt!("module  -- comment\n A.B.C  -- comment\n exports (..);");

            assert_fmt!(
                "module Test exports ( --comment\nfoo);",
                "module Test exports (  --comment\n\tfoo,\n);"
            );

            assert_fmt!("module Test exports (\n\t--comment\n\tfoo,\n);");

            assert_fmt!("module Test exports (\n\tfoo,\n\t-- comment\n\tbar,\n);");
            assert_fmt!(
                "module T exports (foo,  -- comment\n);",
                "module T exports (\n\tfoo,  -- comment\n);"
            );
            assert_fmt!(
                "module T exports (foo,\n  -- comment\n);",
                "module T exports (\n\tfoo,\n\t-- comment\n);"
            );
        }
    }

    mod import_lines {
        macro_rules! assert_fmt {
            ($source:expr) => {{
                assert_fmt!($source, $source, $crate::config::MAX_WIDTH)
            }};
            ($source:expr, $want:expr) => {{
                assert_fmt!($source, $want, $crate::config::MAX_WIDTH)
            }};
            ($source:expr, $want:expr, $max_width:expr) => {{
                let items =
                    $crate::module::gen_import_line(ditto_cst::ImportLine::parse($source).unwrap());
                $crate::test_macros::assert_fmt!(items, $source, $want, $max_width);
            }};
        }

        #[test]
        fn it_formats_import_lines() {
            assert_fmt!("import Foo;");
            assert_fmt!("import Foo.Bar.Baz;");
            assert_fmt!("import Foo as F;");
            assert_fmt!("import (pkg) Foo;");
            assert_fmt!("import (pkg) Foo as F;");
            assert_fmt!("import (foo-bar) Foo as F;");
            assert_fmt!("import Foo (\n\tfoo,\n);");
            assert_fmt!("import Foo (\n\tfoo,\n\tbar,\n);");
            assert_fmt!("import Foo (\n\tfoo,\n\tBar(..),\n);");
            assert_fmt!("import (pkg) Foo (\n\tfoo,\n\tBar(..),\n);");
            assert_fmt!("import  -- comment\n (pkg) Foo;");
            assert_fmt!("import Foo (\n\tBar(  -- comment\n\t\t..\n\t),\n);");
        }
    }
}
