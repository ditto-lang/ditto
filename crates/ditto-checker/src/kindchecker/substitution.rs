use ditto_ast::{Kind, ModuleConstructor, Type};
use non_empty_vec::NonEmpty;
use std::collections::HashMap;

#[derive(Default)]
#[repr(transparent)]
pub struct Substitution(HashMap<usize, Kind>);

impl Substitution {
    pub fn insert(&mut self, var: usize, kind: Kind) {
        self.0.insert(var, kind);
    }
    pub fn apply(&self, kind: Kind) -> Kind {
        match kind {
            // NOTE: avoid using `..` in these patterns so that we're forced
            // to update this logic along with any changes to [Kind]
            Kind::Variable(var) => {
                if let Some(kind) = self.0.get(&var) {
                    // NOTE: substitution proceeds to a fixed point
                    // (i.e. recursively),
                    // which is why we need an occurs check during unification!
                    self.apply(kind.clone())
                } else {
                    kind
                }
            }
            Kind::Function { parameters } => Kind::Function {
                parameters: unsafe {
                    NonEmpty::new_unchecked(
                        parameters
                            .iter()
                            .cloned()
                            .map(|param| self.apply(param))
                            .collect(),
                    )
                },
            },
            Kind::Type => Kind::Type,
            Kind::Row => Kind::Row,
        }
    }
    pub fn apply_type(&self, ast_type: Type) -> Type {
        match ast_type {
            // NOTE: avoid using `..` in these patterns so that we're forced
            // to update this logic along with any changes to [Type]
            Type::Variable {
                variable_kind,
                var,
                source_name,
            } => Type::Variable {
                variable_kind: self.apply(variable_kind),
                var,
                source_name,
            },
            Type::Call {
                box function,
                arguments,
            } => Type::Call {
                function: Box::new(self.apply_type(function)),
                arguments: unsafe {
                    NonEmpty::new_unchecked(
                        arguments
                            .iter()
                            .cloned()
                            .map(|arg| self.apply_type(arg))
                            .collect(),
                    )
                },
            },
            Type::Function {
                parameters,
                box return_type,
            } => Type::Function {
                parameters: parameters
                    .into_iter()
                    .map(|param| self.apply_type(param))
                    .collect(),
                return_type: Box::new(self.apply_type(return_type)),
            },
            Type::Constructor {
                constructor_kind,
                canonical_value,
                source_value,
            } => Type::Constructor {
                constructor_kind: self.apply(constructor_kind),
                canonical_value,
                source_value,
            },
            Type::PrimConstructor(prim_type) => Type::PrimConstructor(prim_type),
            Type::RecordClosed { kind, row } => Type::RecordClosed {
                kind: self.apply(kind),
                row: row
                    .into_iter()
                    .map(|(label, t)| (label, self.apply_type(t)))
                    .collect(),
            },
            Type::RecordOpen {
                kind,
                var,
                row,
                source_name,
            } => Type::RecordOpen {
                kind: self.apply(kind),
                var,
                source_name,
                row: row
                    .into_iter()
                    .map(|(label, t)| (label, self.apply_type(t)))
                    .collect(),
            },
        }
    }
    pub fn apply_constructor(&self, constructor: ModuleConstructor) -> ModuleConstructor {
        let ModuleConstructor {
            doc_comments,
            doc_position,
            constructor_name_span,
            fields,
            return_type,
            return_type_name,
        } = constructor;

        ModuleConstructor {
            doc_comments,
            doc_position,
            constructor_name_span,
            fields: fields.into_iter().map(|t| self.apply_type(t)).collect(),
            return_type: self.apply_type(return_type),
            return_type_name,
        }
    }
}
