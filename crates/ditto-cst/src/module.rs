use crate::{
    AsKeyword, Comment, DoubleDot, Equals, ExportsKeyword, Expression, ForeignKeyword,
    ImportKeyword, ModuleKeyword, ModuleName, Name, PackageName, Parens, ParensList1, Pipe,
    ProperName, Semicolon, Type, TypeAnnotation, TypeKeyword,
};
use std::iter;

/// A ditto (source) module.
#[derive(Debug, Clone)]
pub struct Module {
    /// The module header declares the module's name and exports.
    pub header: Header,
    /// Things that this module depends on from other modules.
    pub imports: Vec<ImportLine>,
    /// Type and value definitions.
    pub declarations: Vec<Declaration>,
    /// Any dangling comments that come after the last declaration.
    pub trailing_comments: Vec<Comment>,
}

/// `module Some.Module exports (..);`
#[derive(Debug, Clone)]
pub struct Header {
    /// `module`
    pub module_keyword: ModuleKeyword,
    /// `Some.Module`
    pub module_name: ModuleName,
    /// `exports`
    pub exports_keyword: ExportsKeyword,
    /// `(..)` or `(Foo, Bar(..), baz)`
    pub exports: Exports,
    /// `;`
    pub semicolon: Semicolon,
}

/// `(..)`
pub type Everything = Parens<DoubleDot>;

/// A list of things to be exported.
#[derive(Debug, Clone)]
pub enum Exports {
    /// `(..)`
    Everything(Everything),
    /// `(Foo, Bar(..), baz)`
    List(Box<ParensList1<Export>>),
}

/// An item in an [Exports] list.
#[derive(Debug, Clone)]
pub enum Export {
    /// `foo`
    Value(Name),
    /// `Foo` or `Foo(..)`
    Type(ProperName, Option<Everything>),
}

/// `import (some_package) Some.Module as Alias (..);`
#[derive(Debug, Clone)]
pub struct ImportLine {
    /// `import`
    pub import_keyword: ImportKeyword,
    /// `(some_package)`
    pub package: Option<Parens<PackageName>>,
    /// `Some.Module`
    pub module_name: ModuleName,
    /// `as Alias`
    pub alias: Option<(AsKeyword, ProperName)>,
    /// `(..)` or `(Foo, Bar(..), baz)`
    pub imports: Option<ImportList>,
    /// `;`
    pub semicolon: Semicolon,
}

/// A list of things to be imported.
///
/// `(Foo, Bar(..), baz)`
#[derive(Debug, Clone)]
pub struct ImportList(pub ParensList1<Import>);

/// An item in an [Import] list.
#[derive(Debug, Clone)]
pub enum Import {
    /// `foo`
    Value(Name),
    /// `Foo` or `Foo(..)`
    Type(ProperName, Option<Everything>),
}

/// Declarations are the body of a module.
#[derive(Debug, Clone)]
pub enum Declaration {
    /// Binding an expression to a top-level name.
    Value(Box<ValueDeclaration>),
    /// Introducing a new type.
    Type(Box<TypeDeclaration>),
    /// An FFI value.
    ForeignValue(Box<ForeignValueDeclaration>),
}

/// Binding an expression to a top-level name.
///
/// ```ditto
/// name : type = expression;
/// ```
#[derive(Debug, Clone)]
pub struct ValueDeclaration {
    /// Name of this value.
    pub name: Name,
    /// Optional type of the value.
    pub type_annotation: Option<TypeAnnotation>,
    /// `=`
    pub equals: Equals,
    /// The value definition itself.
    pub expression: Expression,
    /// `;`
    pub semicolon: Semicolon,
}

/// Introducing a new type.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum TypeDeclaration {
    /// ```ditto
    /// type Maybe(a) =
    ///   | Just(a)
    ///   | Nothing;
    /// ```
    WithConstructors {
        /// `type`
        type_keyword: TypeKeyword,
        /// The name of this type, e.g. `Maybe`.
        type_name: ProperName,
        /// Optional parameters for this type.
        type_variables: Option<ParensList1<Name>>,
        /// `=`
        equals: Equals,
        /// The first constructor (there must be at least one for a type declaration).
        ///
        /// This first constructor has an optional leading `|`.
        head_constructor: Constructor<Option<Pipe>>,
        /// The remaining type constructors.
        tail_constructors: Vec<Constructor>,
        /// `;`
        semicolon: Semicolon,
    },
    /// Types may also be introduced _without_ constructors, if they are to be
    /// constructed via the FFI.
    ///
    /// ```ditto
    /// type Maybe(a);
    /// ```
    WithoutConstructors {
        /// `type`
        type_keyword: TypeKeyword,
        /// The name of this type, e.g. `Maybe`.
        type_name: ProperName,
        /// Optional parameters for this type.
        type_variables: Option<ParensList1<Name>>,
        /// `;`
        semicolon: Semicolon,
    },
}

impl TypeDeclaration {
    /// Get `type_keyword`.
    pub fn type_keyword(&self) -> &TypeKeyword {
        match self {
            Self::WithConstructors { type_keyword, .. } => type_keyword,
            Self::WithoutConstructors { type_keyword, .. } => type_keyword,
        }
    }
    /// Get `type_name`.
    pub fn type_name(&self) -> &ProperName {
        match self {
            Self::WithConstructors { type_name, .. } => type_name,
            Self::WithoutConstructors { type_name, .. } => type_name,
        }
    }
    /// Get `type_variables`.
    pub fn type_variables(&self) -> &Option<ParensList1<Name>> {
        match self {
            Self::WithConstructors { type_variables, .. } => type_variables,
            Self::WithoutConstructors { type_variables, .. } => type_variables,
        }
    }
    /// Iterate through constructors.
    pub fn iter_constructors(self) -> Box<dyn iter::Iterator<Item = Constructor<Option<Pipe>>>> {
        match self {
            Self::WithoutConstructors { .. } => Box::new(iter::empty()),
            Self::WithConstructors {
                head_constructor,
                tail_constructors,
                ..
            } => Box::new(
                iter::once(Constructor {
                    pipe: head_constructor.pipe,
                    constructor_name: head_constructor.constructor_name,
                    fields: head_constructor.fields,
                })
                .chain(tail_constructors.into_iter().map(|ctor| Constructor {
                    pipe: Some(ctor.pipe),
                    constructor_name: ctor.constructor_name,
                    fields: ctor.fields,
                })),
            ),
        }
    }
}

/// A type constructor, like `Just` or `Nothing`.
#[derive(Debug, Clone)]
pub struct Constructor<P = Pipe> {
    /// `|`
    pub pipe: P,
    /// `Just`
    pub constructor_name: ProperName,
    /// Optional type fields for this constructor.
    pub fields: Option<ParensList1<Type>>,
}

/// A foreign value import.
///
/// ```ditto
/// foreign foo : Nat;
/// ```
#[derive(Debug, Clone)]
pub struct ForeignValueDeclaration {
    /// `foreign`
    pub foreign_keyword: ForeignKeyword,
    /// The name of the value being imported.
    pub name: Name,
    /// The type of the value being imported.
    pub type_annotation: TypeAnnotation,
    /// `;`
    pub semicolon: Semicolon,
}
