use ditto_ast as ast;

pub enum Located {
    ValueDeclarationName {
        name: ast::Name,
        module_value: ast::ModuleValue,
    },
    LocalVariable {
        span: ast::Span,
        variable_type: ast::Type,
        variable: ast::Name,
    },
    ForeignVariable {
        span: ast::Span,
        variable_type: ast::Type,
        variable: ast::Name,
    },
    ImportedVariable {
        span: ast::Span,
        variable_type: ast::Type,
        variable: ast::FullyQualifiedName,
    },
    UnitLiteral {
        span: ast::Span,
        value_type: ast::Type,
    },
    TrueLiteral {
        span: ast::Span,
        value_type: ast::Type,
    },
    FalseLiteral {
        span: ast::Span,
        value_type: ast::Type,
    },
    StringLiteral {
        span: ast::Span,
        value_type: ast::Type,
    },
    IntLiteral {
        span: ast::Span,
        value_type: ast::Type,
    },
    FloatLiteral {
        span: ast::Span,
        value_type: ast::Type,
    },
}

pub fn locate(_source: &str, module: ast::Module, offset: usize) -> Option<Located> {
    // Search top-level value declarations
    for (name, module_value) in module.values {
        if module_value.name_span.contains(offset) {
            return Some(Located::ValueDeclarationName { name, module_value });
        }
        if module_value.expression.get_span().contains(offset) {
            return locate_expression(module_value.expression, offset);
        }
    }
    None
}

fn locate_expression(expr: ast::Expression, offset: usize) -> Option<Located> {
    match expr {
        ast::Expression::LocalVariable {
            span,
            variable_type,
            variable,
        } => Some(Located::LocalVariable {
            span,
            variable_type,
            variable,
        }),
        ast::Expression::ForeignVariable {
            span,
            variable_type,
            variable,
        } => Some(Located::ForeignVariable {
            span,
            variable_type,
            variable,
        }),
        ast::Expression::ImportedVariable {
            span,
            variable_type,
            variable,
        } => Some(Located::ImportedVariable {
            span,
            variable_type,
            variable,
        }),
        ast::Expression::Function {
            binders: _, // TODO handle patterns
            box body,
            ..
        } => {
            if body.get_span().contains(offset) {
                return locate_expression(body, offset);
            }
            None
        }
        ast::Expression::Call {
            box function,
            arguments,
            ..
        } => {
            if function.get_span().contains(offset) {
                return locate_expression(function, offset);
            }
            for ast::Argument::Expression(arg) in arguments {
                if arg.get_span().contains(offset) {
                    return locate_expression(arg, offset);
                }
            }
            None
        }
        ast::Expression::If {
            box condition,
            box true_clause,
            box false_clause,
            ..
        } => {
            if condition.get_span().contains(offset) {
                return locate_expression(condition, offset);
            }
            if true_clause.get_span().contains(offset) {
                return locate_expression(true_clause, offset);
            }
            if false_clause.get_span().contains(offset) {
                return locate_expression(false_clause, offset);
            }
            None
        }
        ast::Expression::Match {
            box expression,
            arms,
            ..
        } => {
            if expression.get_span().contains(offset) {
                return locate_expression(expression, offset);
            }
            for (_arm_pattern, arm_expression) in arms {
                if arm_expression.get_span().contains(offset) {
                    return locate_expression(arm_expression, offset);
                }
            }
            None
        }
        ast::Expression::LocalConstructor { .. } => None, // TODO
        ast::Expression::ImportedConstructor { .. } => None, // TODO
        ast::Expression::Effect { effect, .. } => locate_effect(effect, offset),
        ast::Expression::RecordAccess { box target, .. } => {
            if target.get_span().contains(offset) {
                return locate_expression(target, offset);
            }
            None
        }
        ast::Expression::RecordUpdate {
            box target, fields, ..
        } => {
            if target.get_span().contains(offset) {
                return locate_expression(target, offset);
            }
            for (_name, expression) in fields {
                if expression.get_span().contains(offset) {
                    return locate_expression(expression, offset);
                }
            }
            None
        }
        ast::Expression::Let {
            declaration,
            box expression,
            ..
        } => {
            if declaration.expression.get_span().contains(offset) {
                return locate_expression(*declaration.expression, offset);
            }
            if expression.get_span().contains(offset) {
                return locate_expression(expression, offset);
            }
            None
        }
        ast::Expression::Record { fields, .. } => {
            for (_name, expression) in fields {
                if expression.get_span().contains(offset) {
                    return locate_expression(expression, offset);
                }
            }
            None
        }
        ast::Expression::Array { elements, .. } => {
            for element in elements {
                if element.get_span().contains(offset) {
                    return locate_expression(element, offset);
                }
            }
            None
        }
        ast::Expression::True { span, value_type } => {
            Some(Located::TrueLiteral { span, value_type })
        }
        ast::Expression::False { span, value_type } => {
            Some(Located::FalseLiteral { span, value_type })
        }
        ast::Expression::String {
            span, value_type, ..
        } => Some(Located::StringLiteral { span, value_type }),
        ast::Expression::Int {
            span, value_type, ..
        } => Some(Located::IntLiteral { span, value_type }),
        ast::Expression::Float {
            span, value_type, ..
        } => Some(Located::FloatLiteral { span, value_type }),
        ast::Expression::Unit { span, value_type } => {
            Some(Located::UnitLiteral { span, value_type })
        }
    }
}

fn locate_effect(eff: ast::Effect, offset: usize) -> Option<Located> {
    match eff {
        ast::Effect::Bind {
            box expression,
            box rest,
            ..
        } => {
            if expression.get_span().contains(offset) {
                return locate_expression(expression, offset);
            }
            locate_effect(rest, offset)
        }
        ast::Effect::Let {
            pattern: _,
            box expression,
            box rest,
        } => {
            if expression.get_span().contains(offset) {
                return locate_expression(expression, offset);
            }
            locate_effect(rest, offset)
        }
        ast::Effect::Expression {
            box expression,
            rest,
            ..
        } => {
            if expression.get_span().contains(offset) {
                return locate_expression(expression, offset);
            }
            rest.and_then(|box rest| locate_effect(rest, offset))
        }
        ast::Effect::Return { box expression } => locate_expression(expression, offset),
    }
}
