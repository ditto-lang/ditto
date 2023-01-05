use crate::{partial_parse_header, partial_parse_header_and_imports};

#[test]
fn it_parses_partial_headers() {
    partial_parse_header("module Foo exports (..)").unwrap();
    partial_parse_header("module Foo.Bar exports (foo, Bar, Baz(..))").unwrap();
    partial_parse_header("-- comment\nmodule Foo exports (foo, Bar, Baz(..))").unwrap();
    partial_parse_header("-- comment\nmodule Foo exports (foo, Bar, Baz(..)) import M five = 5")
        .unwrap();

    partial_parse_header("module Foo exports (..) ;\n{}()").unwrap();
}

#[test]
fn it_parses_partial_header_and_imports() {
    partial_parse_header_and_imports("module Foo exports (..) import Bar import Baz (baz)")
        .unwrap();
    partial_parse_header_and_imports(
        "-- comment\nmodule Foo exports (..) import (pkg) Bar import Baz (baz)",
    )
    .unwrap();

    let (_, imports) = partial_parse_header_and_imports(
        "module Foo exports (..) import (pkg) Bar import Baz (baz) ;\ngarbage{}()",
    )
    .unwrap();
    assert_eq!(imports.len(), 2);

    let (_, imports) =
        partial_parse_header_and_imports("module Foo exports (..) imports Bar").unwrap();
    assert_eq!(imports.len(), 0);

    let (_, imports) =
        partial_parse_header_and_imports("module Foo exports (..) imports Bar import Baz \n;{}")
            .unwrap();
    assert_eq!(imports.len(), 1);
}
