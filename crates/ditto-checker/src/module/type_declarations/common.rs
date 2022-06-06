use crate::{
    kindchecker::{merge_references, TypeReferences},
    result::{Result, TypeError, Warnings},
};
use ditto_ast::{self as ast, ModuleConstructors};
use ditto_cst::{self as cst, Span, TypeAliasDeclaration, TypeDeclaration};

#[derive(Clone)]
pub enum TypeDeclarationLike {
    TypeDeclaration(TypeDeclaration),
    TypeAliasDeclaration(TypeAliasDeclaration),
}

impl TypeDeclarationLike {
    pub fn type_name(&self) -> &cst::ProperName {
        match self {
            Self::TypeDeclaration(type_decl) => type_decl.type_name(),
            Self::TypeAliasDeclaration(type_alias_decl) => &type_alias_decl.type_name,
        }
    }
    pub fn type_name_str(&self) -> &str {
        match self {
            Self::TypeDeclaration(type_decl) => &type_decl.type_name().0.value,
            Self::TypeAliasDeclaration(type_alias_decl) => &type_alias_decl.type_name.0.value,
        }
    }
}

/// It's easier to store these things in a mutable struct than return then from
/// every function.
pub struct Outputs {
    pub type_references: TypeReferences,
    pub warnings: Warnings,
}

impl Outputs {
    pub fn new() -> Self {
        Self {
            type_references: TypeReferences::new(),
            warnings: Warnings::new(),
        }
    }

    pub fn extend(&mut self, warnings: Warnings, type_references: TypeReferences) {
        self.type_references =
            merge_references(std::mem::take(&mut self.type_references), type_references);
        self.warnings.extend(warnings);
    }
}

pub fn check_duplicate_type_constructor(
    module_constructors: &mut ModuleConstructors,
    constructor_name: &ast::ProperName,
    constructor_name_span: Span,
) -> Result<()> {
    if let Some(previous) = module_constructors.remove(constructor_name) {
        let (previous_constructor, duplicate_constructor) =
            if previous.constructor_name_span.start_offset < constructor_name_span.start_offset {
                (previous.constructor_name_span, constructor_name_span)
            } else {
                (constructor_name_span, previous.constructor_name_span)
            };
        return Err(TypeError::DuplicateTypeConstructor {
            previous_constructor,
            duplicate_constructor,
        });
    }
    Ok(())
}
