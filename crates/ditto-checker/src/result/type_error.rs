use ditto_ast::{
    Kind, ModuleName, Name, PackageName, ProperName, Qualified, QualifiedName, QualifiedProperName,
    Span, Type,
};
use miette::{Diagnostic, NamedSource, SourceSpan};
use std::collections::HashSet;
use thiserror::Error;

/// A fatal error encountered during type-checking or kind-checking.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum TypeError {
    UnknownVariable {
        span: Span,
        variable: QualifiedName,
        names_in_scope: HashSet<QualifiedName>,
    },
    UnknownTypeVariable {
        span: Span,
        variable: Name,
    },
    UnknownConstructor {
        span: Span,
        constructor: QualifiedProperName,
        ctors_in_scope: HashSet<QualifiedProperName>,
    },
    UnknownTypeConstructor {
        span: Span,
        constructor: QualifiedProperName,
    },
    NotAFunction {
        span: Span,
        actual_type: Type,
    },
    TypeNotAFunction {
        span: Span,
        actual_kind: Kind,
    },
    ArgumentLengthMismatch {
        function_span: Span,
        wanted: usize,
        got: usize,
    },
    TypeArgumentLengthMismatch {
        function_span: Span,
        wanted: usize,
        got: usize,
    },
    InfiniteType {
        span: Span,
        var: usize,
        infinite_type: Type,
    },
    InfiniteKind {
        span: Span,
        var: usize,
        infinite_kind: Kind,
    },
    TypesNotEqual {
        span: Span,
        expected: Type,
        actual: Type,
    },
    KindsNotEqual {
        span: Span,
        expected: Kind,
        actual: Kind,
    },
    PackageNotFound {
        span: Span,
        package_name: PackageName,
        // TODO suggestions?
    },
    ModuleNotFound {
        span: Span,
        package_name: Option<PackageName>,
        module_name: ModuleName,
        // TODO suggestions?
    },
    UnknownValueExport {
        span: Span,
        name: Name,
    },
    UnknownTypeExport {
        span: Span,
        type_name: ProperName,
    },
    UnknownValueImport {
        span: Span,
        name: Name,
    },
    UnknownTypeImport {
        span: Span,
        type_name: ProperName,
    },
    NoVisibleConstructors {
        span: Span,
        type_name: ProperName,
    },
    DuplicateImportLine {
        previous_import_line: Span,
        duplicate_import_line: Span,
    },
    DuplicateImportModule {
        previous_import_module: Span,
        duplicate_import_module: Span,
        proper_name: ProperName,
    },
    DuplicateFunctionBinder {
        previous_binder: Span,
        duplicate_binder: Span,
    },
    DuplicatePatternBinder {
        previous_binder: Span,
        duplicate_binder: Span,
    },
    DuplicateValueDeclaration {
        previous_declaration: Span,
        duplicate_declaration: Span,
    },
    DuplicateTypeDeclaration {
        previous_declaration: Span,
        duplicate_declaration: Span,
    },
    DuplicateTypeConstructor {
        previous_constructor: Span,
        duplicate_constructor: Span,
    },
    DuplicateTypeDeclarationVariable {
        previous_variable: Span,
        duplicate_variable: Span,
    },
    ReboundImportType {
        previous_binding: Span,
        new_binding: Span,
        type_name: QualifiedProperName,
    },
    ReboundImportConstructor {
        previous_binding: Span,
        new_binding: Span,
        constructor_name: QualifiedProperName,
    },
    ReboundImportValue {
        previous_binding: Span,
        new_binding: Span,
        variable: QualifiedName,
    },
}

impl TypeError {
    /// Convert a [TypeError] to a pretty error report.
    pub fn into_report(self, source_name: impl AsRef<str>, source: String) -> TypeErrorReport {
        let input = NamedSource::new(source_name, source);
        match self {
            Self::UnknownVariable {
                span,
                variable,
                names_in_scope,
            } => {
                let location = span_to_source_span(span);
                if names_in_scope.is_empty() {
                    TypeErrorReport::UnknownVariable { input, location }
                } else if let Some(suggestion) = find_suggestion(variable, names_in_scope) {
                    TypeErrorReport::UnknownVariableWithSuggestion {
                        input,
                        location,
                        suggestion,
                    }
                } else {
                    TypeErrorReport::UnknownVariable { input, location }
                }
            }
            Self::UnknownConstructor {
                span,
                constructor,
                ctors_in_scope,
            } => {
                let location = span_to_source_span(span);
                if ctors_in_scope.is_empty() {
                    TypeErrorReport::UnknownConstructor { input, location }
                } else if let Some(suggestion) = find_suggestion(constructor, ctors_in_scope) {
                    TypeErrorReport::UnknownConstructorWithSuggestion {
                        input,
                        location,
                        suggestion,
                    }
                } else {
                    TypeErrorReport::UnknownConstructor { input, location }
                }
            }
            Self::UnknownTypeVariable { span, .. } => TypeErrorReport::UnknownTypeVariable {
                input,
                location: span_to_source_span(span),
            },
            Self::UnknownTypeConstructor { span, .. } => TypeErrorReport::UnknownTypeConstructor {
                input,
                location: span_to_source_span(span),
            },
            Self::TypesNotEqual {
                span,
                expected,
                actual,
            } => TypeErrorReport::UnificationError {
                input,
                location: span_to_source_span(span),
                expected: expected.debug_render(),
                actual: actual.debug_render(),
            },

            Self::KindsNotEqual {
                span,
                expected,
                actual,
            } => TypeErrorReport::KindUnificationError {
                input,
                location: span_to_source_span(span),
                expected: expected.debug_render(),
                actual: actual.debug_render(),
            },
            Self::InfiniteType { span, .. } => TypeErrorReport::InfiniteType {
                input,
                location: span_to_source_span(span),
            },
            Self::InfiniteKind { span, .. } => TypeErrorReport::InfiniteKind {
                input,
                location: span_to_source_span(span),
            },
            Self::ModuleNotFound {
                span,
                package_name: Some(package_name),
                ..
            } => TypeErrorReport::ModuleNotFoundInPackage {
                input,
                location: span_to_source_span(span),
                package_name: package_name.to_string(),
            },
            Self::ModuleNotFound { span, .. } => TypeErrorReport::ModuleNotFound {
                input,
                location: span_to_source_span(span),
            },
            Self::PackageNotFound { span, package_name } => TypeErrorReport::PackageNotFound {
                input,
                location: span_to_source_span(span),
                package_name: package_name.to_string(),
            },
            Self::NotAFunction { span, actual_type } => TypeErrorReport::NotAFunction {
                input,
                location: span_to_source_span(span),
                expression_type: actual_type.debug_render(),
            },
            Self::TypeNotAFunction { span, .. } => TypeErrorReport::TypeNotAFunction {
                input,
                location: span_to_source_span(span),
            },
            Self::ArgumentLengthMismatch {
                function_span,
                wanted,
                ..
            } => TypeErrorReport::ArgumentLengthMismatch {
                input,
                function_location: span_to_source_span(function_span),
                wanted_arguments: match wanted {
                    0 => String::from("no arguments"),
                    1 => String::from("1 argument"),
                    n => format!("{} arguments", n),
                },
            },
            Self::TypeArgumentLengthMismatch {
                function_span,
                wanted,
                ..
            } => TypeErrorReport::TypeArgumentLengthMismatch {
                input,
                function_location: span_to_source_span(function_span),
                wanted_parameters: match wanted {
                    0 => String::from("no type parameters"),
                    1 => String::from("1 type parameter"),
                    n => format!("{} type parameters", n),
                },
            },
            Self::UnknownValueExport { span, .. } => TypeErrorReport::UnknownValueExport {
                input,
                location: span_to_source_span(span),
            },
            Self::UnknownTypeExport { span, .. } => TypeErrorReport::UnknownTypeExport {
                input,
                location: span_to_source_span(span),
            },
            Self::UnknownValueImport { span, .. } => TypeErrorReport::UnknownValueImport {
                input,
                location: span_to_source_span(span),
            },
            Self::UnknownTypeImport { span, .. } => TypeErrorReport::UnknownTypeImport {
                input,
                location: span_to_source_span(span),
            },
            Self::NoVisibleConstructors { span, type_name } => {
                TypeErrorReport::NoVisibleConstructors {
                    input,
                    location: span_to_source_span(span),
                    type_name: type_name.0,
                }
            }
            Self::DuplicateImportLine {
                previous_import_line,
                duplicate_import_line,
            } => TypeErrorReport::DuplicateImportLine {
                input,
                previous_line: span_to_source_span(previous_import_line),
                duplicate_line: span_to_source_span(duplicate_import_line),
            },
            Self::DuplicateImportModule {
                previous_import_module,
                duplicate_import_module,
                proper_name,
            } => TypeErrorReport::DuplicateImportModule {
                input,
                previous_import: span_to_source_span(previous_import_module),
                duplicate_import: span_to_source_span(duplicate_import_module),
                module_name: proper_name.0,
            },
            Self::DuplicateFunctionBinder {
                previous_binder,
                duplicate_binder,
            } => TypeErrorReport::DuplicateFunctionBinder {
                input,
                previous_parameter: span_to_source_span(previous_binder),
                shadowing_parameter: span_to_source_span(duplicate_binder),
            },
            Self::DuplicatePatternBinder {
                previous_binder,
                duplicate_binder,
            } => TypeErrorReport::DuplicatePatternBinder {
                input,
                previous_parameter: span_to_source_span(previous_binder),
                shadowing_parameter: span_to_source_span(duplicate_binder),
            },
            Self::DuplicateValueDeclaration {
                previous_declaration,
                duplicate_declaration,
            } => TypeErrorReport::DuplicateValueDeclaration {
                input,
                previous_definition: span_to_source_span(previous_declaration),
                duplicate_definition: span_to_source_span(duplicate_declaration),
            },
            Self::DuplicateTypeDeclaration {
                previous_declaration,
                duplicate_declaration,
            } => TypeErrorReport::DuplicateTypeDeclaration {
                input,
                previous_type: span_to_source_span(previous_declaration),
                duplicate_type: span_to_source_span(duplicate_declaration),
            },
            Self::DuplicateTypeConstructor {
                previous_constructor,
                duplicate_constructor,
            } => TypeErrorReport::DuplicateTypeConstructor {
                input,
                previous_constructor: span_to_source_span(previous_constructor),
                duplicate_constructor: span_to_source_span(duplicate_constructor),
            },
            Self::DuplicateTypeDeclarationVariable {
                previous_variable,
                duplicate_variable,
            } => TypeErrorReport::DuplicateTypeDeclarationVariable {
                input,
                previous_variable: span_to_source_span(previous_variable),
                duplicate_variable: span_to_source_span(duplicate_variable),
            },
            Self::ReboundImportType {
                previous_binding,
                new_binding,
                type_name,
            } => TypeErrorReport::ReboundImportType {
                input,
                previous_binding: span_to_source_span(previous_binding),
                new_binding: span_to_source_span(new_binding),
                type_name: type_name.to_string(),
            },
            Self::ReboundImportValue {
                previous_binding,
                new_binding,
                variable,
            } => TypeErrorReport::ReboundImportValue {
                input,
                previous_binding: span_to_source_span(previous_binding),
                new_binding: span_to_source_span(new_binding),
                value_name: variable.to_string(),
            },
            Self::ReboundImportConstructor {
                previous_binding,
                new_binding,
                constructor_name,
            } => TypeErrorReport::ReboundImportConstructor {
                input,
                previous_binding: span_to_source_span(previous_binding),
                new_binding: span_to_source_span(new_binding),
                constructor_name: constructor_name.to_string(),
            },
        }
    }
}

/// A pretty [TypeError].
#[derive(Error, Debug, Diagnostic)]
#[allow(missing_docs)]
// Styleguide:
//     - lowercase
//     - backtick anything referring to code.
pub enum TypeErrorReport {
    #[error("unknown variable")]
    #[diagnostic(severity(Error))]
    UnknownVariable {
        #[source_code]
        input: NamedSource,
        #[label("not in scope")]
        location: SourceSpan,
    },
    #[error("unknown variable")]
    #[diagnostic(severity(Error), help("did you mean `{suggestion}`?"))]
    UnknownVariableWithSuggestion {
        #[source_code]
        input: NamedSource,
        #[label("not in scope")]
        location: SourceSpan,
        suggestion: String,
    },
    #[error("unknown constructor")]
    #[diagnostic(severity(Error))]
    UnknownConstructor {
        #[source_code]
        input: NamedSource,
        #[label("not in scope")]
        location: SourceSpan,
    },
    #[error("unknown constructor")]
    #[diagnostic(severity(Error), help("did you mean `{suggestion}`?"))]
    UnknownConstructorWithSuggestion {
        #[source_code]
        input: NamedSource,
        #[label("not in scope")]
        location: SourceSpan,
        suggestion: String,
    },
    #[error("unknown type variable")]
    #[diagnostic(severity(Error))]
    UnknownTypeVariable {
        #[source_code]
        input: NamedSource,
        #[label("not in scope")]
        location: SourceSpan,
        // TODO suggestions?
    },
    #[error("unknown type constructor")]
    #[diagnostic(severity(Error))]
    UnknownTypeConstructor {
        #[source_code]
        input: NamedSource,
        #[label("not in scope")]
        location: SourceSpan,
    },
    #[error("types don't unify")]
    #[diagnostic(severity(Error), help("expected {expected}\ngot {actual}"))]
    UnificationError {
        #[source_code]
        input: NamedSource,
        #[label("here")]
        location: SourceSpan,
        expected: String,
        actual: String,
    },
    #[error("kinds don't unify")]
    #[diagnostic(severity(Error), help("expected {expected}\ngot {actual}"))]
    KindUnificationError {
        #[source_code]
        input: NamedSource,
        #[label("here")]
        location: SourceSpan,
        expected: String,
        actual: String,
    },
    #[error("infinite type")]
    #[diagnostic(severity(Error), help("try adding type annotations?"))]
    InfiniteType {
        #[source_code]
        input: NamedSource,
        #[label("here")]
        location: SourceSpan,
    },
    #[error("infinite kind")]
    #[diagnostic(severity(Error), help("please report how you did this"))]
    InfiniteKind {
        #[source_code]
        input: NamedSource,
        #[label("here")]
        location: SourceSpan,
    },
    #[error("module not found")]
    #[diagnostic(severity(Error))]
    ModuleNotFound {
        #[source_code]
        input: NamedSource,
        #[label("not found")]
        location: SourceSpan,
    },
    #[error("module not found")]
    #[diagnostic(severity(Error))]
    ModuleNotFoundInPackage {
        #[source_code]
        input: NamedSource,
        #[label("`{package_name}` doesn't expose this module?")]
        location: SourceSpan,
        package_name: String,
    },
    #[error("package not found")]
    #[diagnostic(
        severity(Error),
        help("try adding `{package_name}` to your dependencies?")
    )]
    PackageNotFound {
        #[source_code]
        input: NamedSource,
        #[label("not installed")]
        location: SourceSpan,
        package_name: String,
    },
    #[error("duplicate top-level name")]
    #[diagnostic(severity(Error))]
    DuplicateValueDeclaration {
        #[source_code]
        input: NamedSource,
        #[label("previously defined here")]
        previous_definition: SourceSpan,
        #[label("can't be redefined here")]
        duplicate_definition: SourceSpan,
    },
    #[error("expression isn't callable")]
    #[diagnostic(severity(Error), help("expression has type: {expression_type}"))]
    NotAFunction {
        #[source_code]
        input: NamedSource,
        #[label("can't call this")]
        location: SourceSpan,
        expression_type: String,
    },
    #[error("type isn't callable")]
    #[diagnostic(severity(Error))]
    TypeNotAFunction {
        #[source_code]
        input: NamedSource,
        #[label("this type takes no parameters")]
        location: SourceSpan,
    },
    #[error("wrong number of arguments")]
    #[diagnostic(severity(Error))]
    ArgumentLengthMismatch {
        #[source_code]
        input: NamedSource,
        #[label("this expects {wanted_arguments}")]
        function_location: SourceSpan,
        wanted_arguments: String,
    },
    #[error("wrong number of type parameters")]
    #[diagnostic(severity(Error))]
    TypeArgumentLengthMismatch {
        #[source_code]
        input: NamedSource,
        #[label("this expects {wanted_parameters}")]
        function_location: SourceSpan,
        wanted_parameters: String,
    },
    #[error("unknown value export")]
    #[diagnostic(severity(Error))]
    UnknownValueExport {
        #[source_code]
        input: NamedSource,
        #[label("this value isn't defined?")]
        location: SourceSpan,
        // TODO suggestions?
    },
    #[error("unknown type export")]
    #[diagnostic(severity(Error))]
    UnknownTypeExport {
        #[source_code]
        input: NamedSource,
        #[label("this type isn't defined?")]
        location: SourceSpan,
        // TODO suggestions?
    },
    #[error("unknown value import")]
    #[diagnostic(severity(Error))]
    UnknownValueImport {
        #[source_code]
        input: NamedSource,
        #[label("this value isn't exposed?")]
        location: SourceSpan,
        // TODO suggestions?
    },
    #[error("unknown type import")]
    #[diagnostic(severity(Error))]
    UnknownTypeImport {
        #[source_code]
        input: NamedSource,
        #[label("this type isn't exposed?")]
        location: SourceSpan,
        // TODO suggestions?
    },
    #[error("no visible constructors")]
    #[diagnostic(severity(Error))]
    NoVisibleConstructors {
        #[source_code]
        input: NamedSource,
        #[label("`{type_name}` type is private")]
        location: SourceSpan,
        type_name: String,
    },
    #[error("duplicate function parameter")]
    #[diagnostic(severity(Error))]
    DuplicateFunctionBinder {
        #[source_code]
        input: NamedSource,
        #[label("previous parameter")]
        previous_parameter: SourceSpan,
        #[label("name can't be reused here")]
        shadowing_parameter: SourceSpan,
    },
    #[error("duplicate pattern variable")]
    #[diagnostic(severity(Error))]
    DuplicatePatternBinder {
        #[source_code]
        input: NamedSource,
        #[label("previous variable")]
        previous_parameter: SourceSpan,
        #[label("name can't be reused here")]
        shadowing_parameter: SourceSpan,
    },
    #[error("duplicate type declaration")]
    #[diagnostic(severity(Error))]
    DuplicateTypeDeclaration {
        #[source_code]
        input: NamedSource,
        #[label("previously defined here")]
        previous_type: SourceSpan,
        #[label("can't be redefined here")]
        duplicate_type: SourceSpan,
    },
    #[error("duplicate constructor")]
    #[diagnostic(severity(Error))]
    DuplicateTypeConstructor {
        #[source_code]
        input: NamedSource,
        #[label("previously defined here")]
        previous_constructor: SourceSpan,
        #[label("can't be redefined here")]
        duplicate_constructor: SourceSpan,
    },
    #[error("duplicate type variable")]
    #[diagnostic(severity(Error))]
    DuplicateTypeDeclarationVariable {
        #[source_code]
        input: NamedSource,
        #[label("previously introduced here")]
        previous_variable: SourceSpan,
        #[label("can't be reintroduced here")]
        duplicate_variable: SourceSpan,
    },
    #[error("duplicate import")]
    #[diagnostic(severity(Error))]
    DuplicateImportLine {
        #[source_code]
        input: NamedSource,
        #[label("previous import")]
        previous_line: SourceSpan,
        #[label("duplicated here")]
        duplicate_line: SourceSpan,
    },
    #[error("duplicate imports for module `{module_name}`")]
    #[diagnostic(severity(Error), help("try aliasing one of the imports?"))]
    DuplicateImportModule {
        #[source_code]
        input: NamedSource,
        #[label("previous import")]
        previous_import: SourceSpan,
        #[label("imported again here")]
        duplicate_import: SourceSpan,
        module_name: String,
    },
    #[error("value `{value_name}` imported multiple times")]
    #[diagnostic(severity(Error))]
    ReboundImportValue {
        #[source_code]
        input: NamedSource,
        #[label("first imported here")]
        previous_binding: SourceSpan,
        #[label("imported again here")]
        new_binding: SourceSpan,
        value_name: String,
    },
    #[error("type `{type_name}` imported multiple times")]
    #[diagnostic(severity(Error))]
    ReboundImportType {
        #[source_code]
        input: NamedSource,
        #[label("first imported here")]
        previous_binding: SourceSpan,
        #[label("imported again here")]
        new_binding: SourceSpan,
        type_name: String,
    },
    #[error("constructor `{constructor_name}` imported multiple times")]
    #[diagnostic(severity(Error))]
    ReboundImportConstructor {
        #[source_code]
        input: NamedSource,
        #[label("first imported here")]
        previous_binding: SourceSpan,
        #[label("imported again here")]
        new_binding: SourceSpan,
        constructor_name: String,
    },
}

fn find_suggestion<T: std::fmt::Display>(
    needle: Qualified<T>,
    haystack: HashSet<Qualified<T>>,
) -> Option<String> {
    // REVIEW this is quite rough and ready!
    let mut engine: simsearch::SimSearch<String> = simsearch::SimSearch::new();
    for qualified in haystack {
        engine.insert(qualified.to_string(), &qualified.to_string());
    }
    let results = engine.search(&needle.to_string());
    results.first().cloned() // REVIEW arbitrarily taking the first result, can probably improve this?
}

/// Convert our [Span] to a miette [SourceSpan].
fn span_to_source_span(span: Span) -> SourceSpan {
    SourceSpan::from((span.start_offset, span.end_offset - span.start_offset))
}
