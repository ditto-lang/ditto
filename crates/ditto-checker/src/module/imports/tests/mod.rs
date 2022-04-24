mod macros;
use crate::{module::tests::macros::assert_module_err, TypeError, Warning};
use macros::*;

#[test]
fn it_handles_value_imports() {
    assert_modules_ok!(
        r#" 
        module Test exports (..);
        import Foo as F (five);
        another_five : Nat = F.id(five);
        "#,
        warnings = [],
        [r#" 
        module Foo exports (five, id);
        five = 5;
        id = (a) -> a;
        "#],
    );
}

#[test]
fn it_handles_type_imports() {
    assert_modules_ok!(
        r#" 
        module Test exports (..);
        import Data.Maybe as M (Maybe(..));
        maybe_fives : Array(M.Maybe(Nat)) = [M.Just(5), Just(5)];
        "#,
        warnings = [],
        [r#" 
        module Data.Maybe exports (Maybe(..));
        type Maybe(a) = Just(a) | Nothing;
        "#],
    );
}

#[test]
fn it_warns_as_expected() {
    assert_modules_ok!(
        r#"
        module Test exports (..);
        import Foo (five, five);
        my_five = five;
        "#,
        warnings = [Warning::DuplicateValueImport { .. }],
        [r#"
        module Foo exports (five, id);
        five = 5;
        id = (a) -> a;
        "#],
    );

    assert_modules_ok!(
        r#"
        module Test exports (..);
        import Data.Five (Five, Five);
        my_five : Five = Five.five;
        "#,
        warnings = [Warning::DuplicateTypeImport { .. }],
        [r#"
        module Data.Five exports (Five(..), five);
        type Five = Five;
        five = Five;
        "#],
    );

    assert_modules_ok!(
        r#"
        module Test exports (..);
        import Data.Five (Five, Five(..));
        my_five : Five = Five.five;
        "#,
        warnings = [Warning::DuplicateTypeImport { .. }],
        [r#"
        module Data.Five exports (Five(..), five);
        type Five = Five;
        five = Five;
        "#],
    );

    assert_modules_ok!(
        r#"
        module Test exports (..);
        import Data.Five (five);
        my_five = 5;
        "#,
        warnings = [Warning::UnusedImport { .. }],
        [r#"
        module Data.Five exports (five);
        five : Nat = 5;
        "#],
    );

    assert_modules_ok!(
        r#"
        module Test exports (..);
        import Data.Five;
        my_five = 5;
        "#,
        warnings = [Warning::UnusedImport { .. }],
        [r#"
        module Data.Five exports (Five(..));
        type Five = Five;
        "#],
    );

    assert_modules_ok!(
        r#"
        module Test exports (..);
        import Bar;
        type Foo = Foo(Bar.Bar);
        "#,
        warnings = [],
        [r#"
        module Bar exports (..);
        type Bar = Bar;
        "#],
    );

    assert_modules_ok!(
        r#"
        module Test exports (..);
        import (five) Data.Five as F (Five);
        my_five : Five = F.Five;
        another_five : Five = F.five;
        "#,
        warnings = [],
        [],
        five = [r#"
        module Data.Five exports (Five(..), five);
        type Five = Five;
        five = Five;
        "#]
    );
}

#[test]
fn it_errors_as_expected() {
    assert_module_err!(
        r#" 
        module Test exports (..);
        import (wut) Huh (nah);
        five : Float = 5.0;
        "#,
        TypeError::PackageNotFound { .. }
    );

    assert_module_err!(
        r#" 
        module Test exports (..);
        import Huh (nah);
        five : Float = 5.0;
        "#,
        TypeError::ModuleNotFound { .. }
    );

    assert_modules_err!(
        r#" 
        module Test exports (..);
        import Foo (huh);
        "#,
        error = TypeError::UnknownValueImport { .. },
        [r#" 
        module Foo exports (five, id);
        five = 5;
        id = (a) -> a;
        "#],
    );

    assert_modules_err!(
        r#" 
        module Test exports (..);
        import Data.Five (Six);
        "#,
        error = TypeError::UnknownTypeImport { .. },
        [r#" 
        module Data.Five exports (Five);
        type Five = Five;
        "#],
    );

    assert_modules_err!(
        r#" 
        module Test exports (..);
        import Data.Five (Five(..));
        "#,
        error = TypeError::NoVisibleConstructors { .. },
        [r#" 
        module Data.Five exports (Five);
        type Five = Five;
        "#],
    );

    assert_modules_err!(
        r#" 
        module Test exports (..);
        import Foo (A);
        import Bar (A);
        "#,
        error = TypeError::ReboundImportType { .. },
        [
            r#" 
        module Foo exports (A);
        type A = A;
        "#,
            r#" 
        module Bar exports (A);
        type A = A;
        "#
        ],
    );

    assert_modules_err!(
        r#" 
        module Test exports (..);
        import Foo (Foo(..));
        import Bar (Bar(..));
        "#,
        error = TypeError::ReboundImportConstructor { .. },
        [
            r#" 
        module Foo exports (Foo(..));
        type Foo = A;
        "#,
            r#" 
        module Bar exports (Bar(..));
        type Bar = A;
        "#
        ],
    );

    assert_modules_err!(
        r#" 
        module Test exports (..);
        import A (yes);
        import B (yes);
        "#,
        error = TypeError::ReboundImportValue { .. },
        [
            r#" 
        module A exports (yes);
        yes = "yes";
        "#,
            r#" 
        module B exports (yes);
        yes = "yes";
        "#
        ],
    );

    assert_modules_err!(
        r#" 
        module Test exports (..);
        import A;
        import A;
        "#,
        error = TypeError::DuplicateImportLine { .. },
        [r#" 
        module A exports (yes);
        yes = "yes";
        "#],
    );

    assert_modules_err!(
        r#" 
        module Test exports (..);
        import Yes as A;
        import No as A;
        "#,
        error = TypeError::DuplicateImportModule { .. },
        [
            r#" 
        module Yes exports (yes);
        yes = "yes";
        "#,
            r#" 
        module No exports (no);
        no = "no";
        "#
        ],
    );
}
