use ditto_ast::Type;

pub fn wobbly(t: Type) -> Type {
    match t {
        Type::Variable {
            is_rigid: _,
            source_name,
            variable_kind,
            var,
        } => Type::Variable {
            is_rigid: false, // important bit
            source_name,
            variable_kind,
            var,
        },
        Type::RecordOpen {
            is_rigid: _,
            source_name,
            kind,
            var,
            row,
        } => Type::RecordOpen {
            is_rigid: false, // important bit!
            source_name,
            kind,
            var,
            row: row
                .into_iter()
                .map(|(label, t)| (label, wobbly(t)))
                .collect(),
        },
        Type::RecordClosed { kind, row } => Type::RecordClosed {
            kind,
            row: row
                .into_iter()
                .map(|(label, t)| (label, wobbly(t)))
                .collect(),
        },
        Type::Call {
            box function,
            box arguments,
        } => Type::Call {
            function: Box::new(wobbly(function)),
            arguments: Box::new(arguments.map(wobbly)),
        },
        Type::Function {
            parameters,
            box return_type,
        } => Type::Function {
            parameters: parameters.into_iter().map(wobbly).collect(),
            return_type: Box::new(wobbly(return_type)),
        },
        Type::ConstructorAlias {
            constructor_kind,
            canonical_value,
            source_value,
            alias_variables,
            box aliased_type,
        } => Type::ConstructorAlias {
            constructor_kind,
            canonical_value,
            source_value,
            alias_variables,
            aliased_type: Box::new(wobbly(aliased_type)),
        },

        Type::PrimConstructor { .. } | Type::Constructor { .. } => t,
    }
}
