//! This gets gross quite quickly when you start dealing with higher-kinds...
use crate::{
    ast::{ident, Ident},
    render::Render,
    Config,
};
use ditto_ast as ast;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

pub fn generate_declarations(
    config: &Config,
    module_name: &ast::ModuleName,
    exports: &ast::ModuleExports,
) -> String {
    let module = convert_exports(config, module_name, exports);
    let mut accum = String::new();
    module.render(&mut accum);
    accum
}

fn convert_exports(
    config: &Config,
    module_name: &ast::ModuleName,
    exports: &ast::ModuleExports,
) -> DeclarationModule {
    let mut imports = HashMap::new();
    let mut declarations = Vec::new();

    macro_rules! convert_type {
        ($ast_type:expr, $type_from_variable:expr) => {{
            let (converted_type, referenced_modules) =
                convert_type($ast_type, module_name, $type_from_variable);

            imports.extend(referenced_modules.into_iter().map(|module_name| {
                (
                    module_name_to_ident(module_name.clone()),
                    (config.module_name_to_path)(module_name),
                )
            }));

            converted_type
        }};
    }

    for (type_name, exported_type) in exports.types.iter() {
        let mut type_generics = match &exported_type.kind {
            ast::Kind::Type => Vec::new(),
            ast::Kind::Variable(_) => unreachable!(),
            ast::Kind::Function { parameters, .. } => parameters
                .iter()
                .enumerate()
                .map(|(i, _)| mk_type_variable_ident(i))
                .collect(),
        };
        let mut constructor_types = Vec::new();
        for (constructor_name, constructor) in exports.constructors.iter() {
            if constructor.return_type_name == *type_name {
                constructor_types.push({
                    let mut types = vec![Type::StringLiteral(constructor_name.0.clone())];
                    if let ast::Type::Function {
                        parameters: fields, ..
                    } = &constructor.constructor_type
                    {
                        for field in fields {
                            types.push(convert_type!(
                                field,
                                Box::new(|i| mk_type_variable_ident(i).into())
                            ));
                        }
                    }
                    (constructor_name.0.clone(), Type::Tuple(types))
                })
            }
        }
        if cfg!(debug_assertions) {
            // Sort for determinsim
            constructor_types.sort_by(|a, b| a.0.cmp(&b.0));
            type_generics.sort_by(|a, b| a.0.cmp(&b.0));
        }

        let type_name = Ident::from(type_name.clone());
        declarations.push(ExportDeclaration::Type {
            type_name,
            type_generics,
            constructor_types: constructor_types.into_iter().map(|elem| elem.1).collect(),
        });
    }
    let idents_and_types = exports
        .constructors
        .iter()
        .map(|(constructor_name, constructor)| {
            (
                Ident::from(constructor_name.clone()),
                constructor.constructor_type.clone(),
            )
        })
        .chain(exports.values.iter().map(|(value_name, value)| {
            (Ident::from(value_name.clone()), value.value_type.clone())
        }));

    for (ident, ast_type) in idents_and_types {
        if matches!(ast_type, ast::Type::Function { .. }) {
            let function_generics_ref = Rc::new(RefCell::new(HashSet::new()));
            let function_type = convert_type!(
                &ast_type,
                Box::new({
                    let function_generics = function_generics_ref.clone();
                    move |i| {
                        let ident = mk_type_variable_ident(i);
                        function_generics.borrow_mut().insert(ident.clone());
                        ident.into()
                    }
                })
            );

            let mut function_generics =
                function_generics_ref.take().into_iter().collect::<Vec<_>>();

            if cfg!(debug_assertions) {
                // Sort for determinsim
                function_generics.sort_by(|a, b| a.0.cmp(&b.0));
            }

            declarations.push(ExportDeclaration::Function {
                function_name: ident,
                function_generics,
                function_type,
            });
        } else {
            let value_type = convert_type!(&ast_type, Box::new(|_| ident!("never").into()));

            declarations.push(ExportDeclaration::Const {
                value_name: ident,
                value_type,
            });
        }
    }
    let mut imports = imports.into_iter().collect::<Vec<_>>();

    if cfg!(debug_assertions) {
        // Sort for determinism
        imports.sort_by(|a, b| a.0 .0.cmp(&b.0 .0));
        declarations.sort_by(|a, b| a.declaration_name().cmp(b.declaration_name()));
    }

    DeclarationModule {
        imports,
        declarations,
    }
}

fn mk_type_variable_ident(i: usize) -> Ident {
    ident!(format!("T{}", i))
}

fn convert_type(
    ast_type: &ast::Type,
    current_module_name: &ast::ModuleName,
    type_from_variable: Box<dyn Fn(usize) -> Type>,
) -> (Type, HashSet<ast::FullyQualifiedModuleName>) {
    let mut referenced_modules = HashSet::new();
    let converted = convert_type_rec(
        ast_type,
        current_module_name,
        &type_from_variable,
        &mut referenced_modules,
        true,
    );
    (converted, referenced_modules)
}

fn convert_type_rec(
    ast_type: &ast::Type,
    current_module_name: &ast::ModuleName,
    type_from_variable: &dyn Fn(usize) -> Type,
    referenced_modules: &mut HashSet<ast::FullyQualifiedModuleName>,
    // TypeScript doesn't support higher-kinds
    // https://github.com/microsoft/TypeScript/issues/1213
    need_kind_type: bool,
) -> Type {
    match ast_type {
        ast::Type::PrimConstructor(ast::PrimType::String) => ident!("string").into(),
        ast::Type::PrimConstructor(ast::PrimType::Float) => ident!("number").into(),
        ast::Type::PrimConstructor(ast::PrimType::Int) => ident!("number").into(),
        ast::Type::PrimConstructor(ast::PrimType::Array) => {
            if need_kind_type {
                ident!("any").into()
            } else {
                ident!("Array").into()
            }
        }
        ast::Type::PrimConstructor(ast::PrimType::Bool) => ident!("boolean").into(),
        ast::Type::PrimConstructor(ast::PrimType::Unit) => ident!("undefined").into(),

        ast::Type::Variable {
            var, variable_kind, ..
        } => {
            match variable_kind {
                ast::Kind::Type => type_from_variable(*var),
                ast::Kind::Variable(_) => {
                    // If the kind is variable it's not referenced anywhere,
                    // so just add a generic for it
                    type_from_variable(*var)
                }
                ast::Kind::Function { .. } => {
                    // No need to check `need_kind_type` because TypeScript doesn't support
                    // higher-kinded generics,
                    ident!("any").into()
                }
            }
        }
        ast::Type::Constructor {
            canonical_value,
            constructor_kind,
            ..
        } => {
            if need_kind_type && *constructor_kind != ast::Kind::Type {
                return ident!("any").into();
            }
            if canonical_value.module_name.0.is_none()
                && canonical_value.module_name.1 == *current_module_name
            {
                Ident::from(canonical_value.value.clone()).into()
            } else {
                referenced_modules.insert(canonical_value.module_name.clone());
                Ident(format!(
                    "{}.{}",
                    module_name_to_ident(canonical_value.module_name.clone()).0,
                    canonical_value.value.0
                ))
                .into()
            }
        }
        ast::Type::Call {
            box function,
            arguments,
        } => {
            if let ast::Type::Variable { .. } = function {
                return ident!("any").into();
            }

            let converted = convert_type_rec(
                function,
                current_module_name,
                type_from_variable,
                referenced_modules,
                false,
            );
            match converted {
                Type::Ident(applied_type) => {
                    let arguments = arguments
                        .into_iter()
                        .map(|t| {
                            convert_type_rec(
                                t,
                                current_module_name,
                                type_from_variable,
                                referenced_modules,
                                true,
                            )
                        })
                        .collect();
                    Type::Apply {
                        applied_type,
                        arguments,
                    }
                }
                other => unimplemented!("{:?}", other),
            }
        }
        ast::Type::Function {
            parameters,
            box return_type,
        } => {
            let parameters = parameters
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    (
                        Ident(format!("${}", i)),
                        convert_type_rec(
                            t,
                            current_module_name,
                            type_from_variable,
                            referenced_modules,
                            true,
                        ),
                    )
                })
                .collect();
            let return_type = Box::new(convert_type_rec(
                return_type,
                current_module_name,
                type_from_variable,
                referenced_modules,
                true,
            ));
            Type::Function {
                parameters,
                return_type,
            }
        }
    }
}

fn module_name_to_ident(module_name: ast::FullyQualifiedModuleName) -> Ident {
    match module_name {
        (None, module_name) => Ident(module_name.into_string("$")),
        (Some(package_name), module_name) => Ident(format!(
            "{}${}",
            package_name.0.replace('-', "_"),
            module_name.into_string("$")
        )),
    }
}

struct DeclarationModule {
    imports: Vec<(Ident, String)>,
    declarations: Vec<ExportDeclaration>,
}

impl Render for DeclarationModule {
    fn render(&self, accum: &mut String) {
        for (ident, path) in self.imports.iter() {
            accum.push_str(&format!(
                "import * as {ident} from \"{path}\";\n",
                ident = ident.0
            ));
        }
        for decl in self.declarations.iter() {
            decl.render(accum);
            accum.push('\n');
        }
    }
}

enum ExportDeclaration {
    Type {
        type_name: Ident,
        type_generics: Vec<Ident>,
        constructor_types: Vec<Type>,
    },
    Const {
        value_name: Ident,
        value_type: Type,
    },
    Function {
        function_name: Ident,
        function_generics: Vec<Ident>,
        function_type: Type,
    },
}

impl ExportDeclaration {
    fn declaration_name(&self) -> &Ident {
        match self {
            Self::Type { type_name, .. } => type_name,
            Self::Const { value_name, .. } => value_name,
            Self::Function { function_name, .. } => function_name,
        }
    }
}

impl Render for ExportDeclaration {
    fn render(&self, accum: &mut String) {
        match self {
            Self::Type {
                type_name,
                type_generics,
                constructor_types,
            } => {
                accum.push_str("export declare type ");
                accum.push_str(&type_name.0);
                if !type_generics.is_empty() {
                    accum.push('<');
                    let len = type_generics.len();
                    for (i, ident) in type_generics.iter().enumerate() {
                        accum.push_str(&ident.0);
                        if i < len - 1 {
                            accum.push_str(", ");
                        }
                    }
                    accum.push('>');
                }

                accum.push_str(" = ");
                if constructor_types.is_empty() {
                    accum.push_str("any"); // REVIEW
                } else {
                    let len = constructor_types.len();
                    for (i, constructor_type) in constructor_types.iter().enumerate() {
                        constructor_type.render(accum);
                        if i < len - 1 {
                            accum.push_str(" | ");
                        }
                    }
                }
                accum.push(';')
            }
            Self::Const {
                value_name,
                value_type,
            } => {
                accum.push_str("export declare const ");
                accum.push_str(&value_name.0);
                accum.push_str(": ");
                value_type.render(accum);
                accum.push(';');
            }
            Self::Function {
                function_name,
                function_generics,
                function_type,
            } => {
                accum.push_str("export declare function ");
                accum.push_str(&function_name.0);
                if !function_generics.is_empty() {
                    accum.push('<');
                    let len = function_generics.len();
                    for (i, ident) in function_generics.iter().enumerate() {
                        accum.push_str(&ident.0);
                        if i < len - 1 {
                            accum.push_str(", ");
                        }
                    }
                    accum.push('>');
                }
                match function_type {
                    Type::Function {
                        parameters,
                        return_type,
                    } => {
                        render_function_type(parameters, return_type, accum, false);
                    }
                    type_ => {
                        // Shouldn't really happen
                        type_.render(accum);
                    }
                }
                accum.push(';')
            }
        }
    }
}

#[derive(Debug)]
enum Type {
    StringLiteral(String),
    Ident(Ident),
    Apply {
        applied_type: Ident,
        arguments: Vec<Type>,
    },
    Function {
        parameters: Vec<(Ident, Type)>,
        return_type: Box<Type>,
    },
    Tuple(Vec<Type>),
}

impl From<Ident> for Type {
    fn from(ident: Ident) -> Self {
        Self::Ident(ident)
    }
}

impl Render for Type {
    fn render(&self, accum: &mut String) {
        match self {
            Self::StringLiteral(string) => {
                accum.push('"');
                accum.push_str(string);
                accum.push('"');
            }
            Self::Tuple(types) => {
                accum.push('[');
                let types_len = types.len();
                for (i, type_) in types.iter().enumerate() {
                    type_.render(accum);
                    if i < types_len - 1 {
                        accum.push_str(", ");
                    }
                }
                accum.push(']');
            }
            Self::Ident(ident) => ident.render(accum),
            Self::Apply {
                applied_type,
                arguments,
            } => {
                accum.push_str(&applied_type.0);
                accum.push('<');
                let arguments_len = arguments.len();
                for (i, arg_type) in arguments.iter().enumerate() {
                    arg_type.render(accum);
                    if i < arguments_len - 1 {
                        accum.push_str(", ");
                    }
                }
                accum.push('>');
            }
            Self::Function {
                parameters,
                return_type,
            } => {
                render_function_type(parameters, return_type, accum, true);
            }
        }
    }
}

fn render_function_type(
    parameters: &[(Ident, Type)],
    return_type: &Type,
    accum: &mut String,
    arrow_return_type: bool,
) {
    accum.push('(');
    let parameters_len = parameters.len();
    for (i, (parameter_ident, parameter_type)) in parameters.iter().enumerate() {
        accum.push_str(&parameter_ident.0);
        accum.push_str(": ");
        parameter_type.render(accum);
        if i < parameters_len - 1 {
            accum.push_str(", ");
        }
    }
    if arrow_return_type {
        accum.push_str(") => ");
    } else {
        accum.push_str("): ");
    }

    return_type.render(accum);
}
