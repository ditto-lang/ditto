use super::{
    expression::{gen_body_expression, gen_type_annotation},
    helpers::space,
    name::{gen_name, gen_proper_name},
    r#type::gen_type,
    syntax::gen_parens_list1,
    token::{gen_equals, gen_foreign_keyword, gen_pipe, gen_semicolon, gen_type_keyword},
};
use ditto_cst::{
    Constructor, Declaration, ForeignValueDeclaration, Pipe, TypeDeclaration, ValueDeclaration,
};
use dprint_core::formatting::{ir_helpers, PrintItems, Signal};

pub fn gen_declaration(declaration: Declaration) -> PrintItems {
    match declaration {
        Declaration::Value(box value_declaration) => gen_value_declaration(value_declaration),
        Declaration::Type(box type_declaration) => gen_type_declaration(type_declaration),
        Declaration::ForeignValue(box foreign_value_declaration) => {
            gen_foreign_value_declaration(foreign_value_declaration)
        }
    }
}

fn gen_value_declaration(decl: ValueDeclaration) -> PrintItems {
    let mut items = PrintItems::new();
    items.extend(gen_name(decl.name));
    if let Some(type_ann) = decl.type_annotation {
        items.extend(gen_type_annotation(type_ann));
    }
    items.extend(space());
    let equals_has_trailing_comment = decl.equals.0.has_trailing_comment();
    items.extend(gen_equals(decl.equals));
    items.extend(gen_body_expression(
        decl.expression,
        equals_has_trailing_comment,
    ));

    items.extend(gen_semicolon(decl.semicolon));
    items
}

fn gen_type_declaration(type_declaration: TypeDeclaration) -> PrintItems {
    // REVIEW use ir_helpers::gen_separated_values for constructors?
    match type_declaration {
        TypeDeclaration::WithoutConstructors {
            type_keyword,
            type_name,
            type_variables,
            semicolon,
        } => {
            let mut items = PrintItems::new();
            items.extend(gen_type_keyword(type_keyword));
            items.extend(space());
            items.extend(gen_proper_name(type_name));
            if let Some(type_variables) = type_variables {
                items.extend(gen_parens_list1(type_variables, gen_name, false));
            }
            items.extend(gen_semicolon(semicolon));
            items
        }
        TypeDeclaration::WithConstructors {
            type_keyword,
            type_name,
            type_variables,
            equals,
            head_constructor,
            tail_constructors,
            semicolon,
        } => {
            let mut items = PrintItems::new();
            items.extend(gen_type_keyword(type_keyword));
            items.extend(space());
            items.extend(gen_proper_name(type_name));
            if let Some(type_variables) = type_variables {
                items.extend(gen_parens_list1(type_variables, gen_name, false));
            }
            items.extend(space());
            items.extend(gen_equals(equals));
            items.push_signal(Signal::SpaceOrNewLine);

            let mut constructor_items = PrintItems::new();
            if tail_constructors.is_empty() {
                if let Some(false) = head_constructor
                    .pipe
                    .as_ref()
                    .map(|pipe| pipe.0.has_comments())
                {
                    // Drop the pipe if it's present and has no comments
                    constructor_items.extend(gen_constructor(Constructor {
                        pipe: None,
                        constructor_name: head_constructor.constructor_name,
                        fields: head_constructor.fields,
                    }));
                } else {
                    constructor_items.extend(gen_constructor(head_constructor));
                }
            } else {
                constructor_items.push_signal(Signal::ExpectNewLine);
                if head_constructor.pipe.is_none() {
                    constructor_items.push_str("| ");
                }
                constructor_items.extend(gen_constructor(head_constructor));
                constructor_items.push_signal(Signal::NewLine);

                let tail_constructors_len = tail_constructors.len();
                for (i, ctor) in tail_constructors.into_iter().enumerate() {
                    constructor_items.extend(gen_constructor(Constructor {
                        pipe: Some(ctor.pipe),
                        constructor_name: ctor.constructor_name,
                        fields: ctor.fields,
                    }));
                    if i < tail_constructors_len - 1 {
                        constructor_items.push_signal(Signal::NewLine);
                    }
                }
            }

            items.extend(ir_helpers::with_indent(constructor_items));

            items.extend(gen_semicolon(semicolon));
            items
        }
    }
}

fn gen_constructor(ctor: Constructor<Option<Pipe>>) -> PrintItems {
    let mut items = PrintItems::new();
    if let Some(pipe) = ctor.pipe {
        items.extend(gen_pipe(pipe));
        items.extend(space());
    }
    items.extend(gen_proper_name(ctor.constructor_name));
    if let Some(fields) = ctor.fields {
        items.extend(gen_parens_list1(fields, gen_type, false));
    }
    items
}

fn gen_foreign_value_declaration(decl: ForeignValueDeclaration) -> PrintItems {
    let mut items = PrintItems::new();
    items.extend(gen_foreign_keyword(decl.foreign_keyword));
    items.extend(space());
    items.extend(gen_name(decl.name));
    items.extend(gen_type_annotation(decl.type_annotation));
    items.extend(gen_semicolon(decl.semicolon));
    items
}

#[cfg(test)]
mod tests {
    mod type_decls {
        macro_rules! assert_fmt {
            ($source:expr) => {{
                assert_fmt!($source, $source, $crate::config::MAX_WIDTH)
            }};
            ($source:expr, $want:expr) => {{
                assert_fmt!($source, $want, $crate::config::MAX_WIDTH)
            }};
            ($source:expr, $want:expr, $max_width:expr) => {{
                let items = $crate::declaration::gen_type_declaration(
                    ditto_cst::TypeDeclaration::parse($source).unwrap(),
                );
                $crate::test_macros::assert_fmt!(items, $source, $want, $max_width);
            }};
        }

        #[test]
        fn it_formats_type_declarations() {
            assert_fmt!("type Unknown;");
            assert_fmt!("-- comment\ntype Unknown;  -- comment");
            assert_fmt!("type Huh(\n\t-- comment\n\ta,\n);");
            assert_fmt!("type Unit = Unit;");
            assert_fmt!(
                "type Unit = Loooooooooooooooooooooooooooooooooooooooooong;",
                "type Unit =\n\tLoooooooooooooooooooooooooooooooooooooooooong;",
                20
            );
            assert_fmt!("type Unit =\n\t-- comment\n\tUnit;");
            assert_fmt!("type Unit = | Unit;", "type Unit = Unit;");
            assert_fmt!("type AB = A | B;", "type AB =\n\t| A\n\t| B;");
            assert_fmt!("type Maybe(a) =\n\t-- comment\n\t| Just(a)\n\t-- comment\n\t| Nothing;");
        }
    }

    mod value_decls {
        macro_rules! assert_fmt {
            ($source:expr) => {{
                assert_fmt!($source, $source, $crate::config::MAX_WIDTH)
            }};
            ($source:expr, $want:expr) => {{
                assert_fmt!($source, $want, $crate::config::MAX_WIDTH)
            }};
            ($source:expr, $want:expr, $max_width:expr) => {{
                let items = $crate::declaration::gen_value_declaration(
                    ditto_cst::ValueDeclaration::parse($source).unwrap(),
                );
                $crate::test_macros::assert_fmt!(items, $source, $want, $max_width);
            }};
        }

        #[test]
        fn it_formats_value_declarations() {
            assert_fmt!("foo = 5;");
            assert_fmt!("foo: Nat = 5;");
            assert_fmt!("foo: Nat = 5;", "foo: Nat =\n\t5;", 5);
            assert_fmt!("foo: Nat =  -- comment\n\t5;");
            assert_fmt!("foo: Nat =\n\t-- comment\n\t5;");
            assert_fmt!("f: (a, b) -> c =\n\t-- comment\n\t[1, 2, 3, 4, 5];");
            assert_fmt!("f: Dunno =  -- comment\n\t-- comment\n\tbody;");
            assert_fmt!(
                "x = xxxxxxxxxxxxxxxxxxxxxxxxxx;",
                "x =\n\txxxxxxxxxxxxxxxxxxxxxxxxxx;",
                10
            );
            assert_fmt!("to_string = (dunno: Unknown): Maybe(String) -> to_string_impl(\n\tdunno,\n\tJust,\n\tNothing,\n);");
            assert_fmt!("xs: Array(Nat) = [\n\t-- comment\n\t1,\n];");
            assert_fmt!("xs: Array(Nat) =  -- comment\n\t-- comment\n\t[5];");
            assert_fmt!(
                "whytho = looooong(looooong(loooooong(loooooong(5))));",
                "whytho =\n\tlooooong(\n\t\tlooooong(\n\t\t\tloooooong(\n\t\t\t\tloooooong(\n\t\t\t\t\t5,\n\t\t\t\t),\n\t\t\t),\n\t\t),\n\t);",
                5
            );
        }
    }

    mod foreign_decls {
        macro_rules! assert_fmt {
            ($source:expr) => {{
                assert_fmt!($source, $source, $crate::config::MAX_WIDTH)
            }};
            ($source:expr, $want:expr) => {{
                assert_fmt!($source, $want, $crate::config::MAX_WIDTH)
            }};
            ($source:expr, $want:expr, $max_width:expr) => {{
                let items = $crate::declaration::gen_foreign_value_declaration(
                    ditto_cst::ForeignValueDeclaration::parse($source).unwrap(),
                );
                $crate::test_macros::assert_fmt!(items, $source, $want, $max_width);
            }};
        }

        #[test]
        fn it_formats_foreign_value_declarations() {
            assert_fmt!("foreign foo: Nat;");
            assert_fmt!("foreign  --comment\n foo: Nat;");
            assert_fmt!("foreign foo: (\n\t-- comment a,\n) -> b;");
        }
    }
}
