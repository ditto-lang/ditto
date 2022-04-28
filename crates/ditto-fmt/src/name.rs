use super::token::{gen_dot, gen_string_token};
use ditto_cst::{
    ModuleName, Name, PackageName, ProperName, QualifiedName, QualifiedProperName, UnusedName,
};
use dprint_core::formatting::PrintItems;

pub fn gen_module_name(module_name: ModuleName) -> PrintItems {
    let mut items = PrintItems::new();
    for (proper_name, dot) in module_name.init {
        items.extend(gen_proper_name(proper_name));
        // why would u put a comment here
        items.extend(gen_dot(dot));
        // or here
    }
    items.extend(gen_proper_name(module_name.last));
    items
}

pub fn gen_qualified_proper_name(qualified: QualifiedProperName) -> PrintItems {
    let mut items = PrintItems::new();
    if let Some((proper_name, dot)) = qualified.module_name {
        items.extend(gen_proper_name(proper_name));
        // don't put a comment here
        items.extend(gen_dot(dot));
        // or here
    }
    items.extend(gen_proper_name(qualified.value));
    items
}

pub fn gen_qualified_name(qualified: QualifiedName) -> PrintItems {
    let mut items = PrintItems::new();
    if let Some((proper_name, dot)) = qualified.module_name {
        items.extend(gen_proper_name(proper_name));
        // don't put a comment here
        items.extend(gen_dot(dot));
        // or here
    }
    items.extend(gen_name(qualified.value));
    items
}

pub fn gen_proper_name(proper_name: ProperName) -> PrintItems {
    gen_string_token(proper_name.0)
}

pub fn gen_name(name: Name) -> PrintItems {
    gen_string_token(name.0)
}

pub fn gen_unused_name(unused_name: UnusedName) -> PrintItems {
    gen_string_token(unused_name.0)
}

pub fn gen_package_name(package_name: PackageName) -> PrintItems {
    gen_string_token(package_name.0)
}

#[cfg(test)]
mod tests {
    macro_rules! assert_fmt {
        ($source:expr) => {{
            assert_fmt!($source, $source, 80)
        }};
        ($source:expr, $want:expr) => {{
            assert_fmt!($source, $want, 80)
        }};
        ($source:expr, $want:expr, $max_width:expr) => {{
            let items =
                $crate::name::gen_module_name(ditto_cst::ModuleName::parse($source).unwrap());
            $crate::test_macros::assert_fmt!(items, $source, $want, $max_width);
        }};
    }

    #[test]
    fn it_formats_module_names() {
        assert_fmt!("Foo");
        assert_fmt!("Foo.Bar");
        assert_fmt!("Foo. Bar . Baz", "Foo.Bar.Baz");
        assert_fmt!("Foo \n.Bar", "Foo.Bar");
        assert_fmt!("Foo  -- comment\n.Bar");
        assert_fmt!("Foo  -- comment\n.  -- comment\nBar");
    }
}
