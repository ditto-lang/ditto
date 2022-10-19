use crate::{graph::Scc, Expression, Kind, ModuleName, Name, ProperName, Span, Type};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A ditto module.
///
/// A module captures three namespaces: types, constructors and values.
#[derive(Debug, Serialize, Deserialize)]
pub struct Module {
    /// The name of the module, e.g. `Some.Module`.
    ///
    /// When the module is defined in a file, this name should roughly agree with
    /// the path, i.e. `Some.Module` should live at `src/Some/Module.ditto`.
    ///
    /// But modules can be compiled from stdin and other sources, hence this isn't redundant.
    pub module_name: ModuleName,

    /// Things exported by this module, i.e. it's interface.
    pub exports: ModuleExports,

    /// Types defined in this module.
    pub types: ModuleTypes,

    /// Types defined in this module.
    pub constructors: ModuleConstructors,

    /// Top-level values defined within the module.
    ///
    /// The flattened names should form a unique list.
    pub values: ModuleValues,

    /// The topological sort order of `values`.
    pub values_toposort: Vec<Scc<Name>>,
    // REVIEW we could make the `values` and `values_toposort` fields private
    // and expose getter/setter methods, for safety? Might be overkill though...
}

/// The type of `module.types`, for convenience.
pub type ModuleTypes = IndexMap<ProperName, ModuleType>;

/// A type defined by a module.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleType {
    /// Documentation comments (if any).
    pub doc_comments: Vec<String>,
    /// The source location of the [ProperName].
    pub type_name_span: Span,
    /// The kind of this [Type].
    pub kind: Kind,
}

/// The type of `module.constructors`, for convenience.
pub type ModuleConstructors = IndexMap<ProperName, ModuleConstructor>;

/// The type of `module.values`, for convenience.
pub type ModuleValues = IndexMap<Name, ModuleValue>;

/// A value defined by a module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleValue {
    /// Documentation comments (if any).
    pub doc_comments: Vec<String>,
    /// The source location of the [Name].
    pub name_span: Span,
    /// The value expression.
    pub expression: Expression,
}

impl Module {
    /// Returns the topologically sorted module values.
    pub fn values_toposorted(&self) -> Vec<Scc<(Name, Expression)>> {
        self.values_toposort
            .iter()
            .map(|scc| {
                scc.clone().map(|name| {
                    let module_value = self.values.get(&name).cloned().unwrap();
                    (name, module_value.expression)
                })
            })
            .collect()
    }
}

/// A single constructor, e.g. the `Ok` constructor for `Result`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConstructor {
    /// Documentation comments (if any).
    pub doc_comments: Vec<String>,
    /// Where this constructor should appear among other constructors in the docs.
    pub doc_position: usize,
    /// The source location of the [ProperName].
    pub constructor_name_span: Span,
    /// Fields of this constructor.
    ///
    /// For `Ok(a)`, the field is `[a]`.
    pub fields: Vec<Type>,
    /// The type returned when this constructor is applied to its `fields`.
    pub return_type: Type,
    /// The name of the type this constructor belongs to.
    ///
    /// Used for associating `module.constructors` with `module.types`.
    pub return_type_name: ProperName,
}

impl ModuleConstructor {
    /// Return the type of this [ModuleConstructor].
    pub fn get_type(&self) -> Type {
        if self.fields.is_empty() {
            self.return_type.clone()
        } else {
            Type::Function {
                parameters: self.fields.clone(),
                return_type: Box::new(self.return_type.clone()),
            }
        }
    }
}

/// Everything that a module can expose.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModuleExports {
    /// Exposed type constructors.
    pub types: ModuleExportsTypes,
    /// Exposed type constructors.
    pub constructors: ModuleExportsConstructors,
    /// Exposed values.
    pub values: ModuleExportsValues,
}

/// The type of `module_exports.types`, for convenience.
pub type ModuleExportsTypes = IndexMap<ProperName, ModuleExportsType>;

/// A single exposed type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleExportsType {
    /// Documentation comments (if any).
    pub doc_comments: Vec<String>,
    /// Where this type should appear in the docs.
    pub doc_position: usize,
    /// The kind of the exposed type.
    pub kind: Kind,
}

/// The type of `module_exports.constructors`, for convenience.
pub type ModuleExportsConstructors = IndexMap<ProperName, ModuleExportsConstructor>;

/// A single exposed constructor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleExportsConstructor {
    /// Documentation comments (if any).
    pub doc_comments: Vec<String>,
    /// Where this constructor should appear among other constructors in the docs.
    pub doc_position: usize,
    /// The type of the exposed constructor.
    pub constructor_type: Type,
    /// The name of the type this constructor belongs to.
    ///
    /// Used for associating `module_exports.constructors` with `module_exports.types`.
    pub return_type_name: ProperName,
}

/// The type of `module_exports.values`, for convenience.
pub type ModuleExportsValues = IndexMap<Name, ModuleExportsValue>;

/// A single exposed value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleExportsValue {
    /// Documentation comments (if any).
    pub doc_comments: Vec<String>,
    /// Where this value should appear in the docs.
    pub doc_position: usize,
    /// The type of the exposed value.
    pub value_type: Type,
}
