macro_rules! identity_type {
    ($name:expr) => {
        ditto_ast::Type::Function {
            parameters: vec![ditto_ast::Type::Variable {
                variable_kind: ditto_ast::Kind::Type,
                var: 0,
                source_name: Some(ditto_ast::name!($name)),
            }],
            return_type: Box::new(ditto_ast::Type::Variable {
                variable_kind: ditto_ast::Kind::Type,
                var: 0,
                source_name: Some(ditto_ast::name!($name)),
            }),
        }
    };
    () => {
        ditto_ast::Type::Function {
            parameters: vec![ditto_ast::Type::Variable {
                variable_kind: ditto_ast::Kind::Type,
                var: 0,
                source_name: None,
            }],
            return_type: Box::new(ditto_ast::Type::Variable {
                variable_kind: ditto_ast::Kind::Type,
                var: 0,
                source_name: None,
            }),
        }
    };
}

macro_rules! identity_scheme {
    ($name:expr) => {
        Scheme {
            forall: $crate::scheme::SchemeForall::from_iter(vec![0]),
            signature: $crate::tests::identity_type!($name),
        }
    };
    () => {
        Scheme {
            forall: $crate::scheme::SchemeForall::from_iter(vec![0]),
            signature: $crate::tests::identity_type!(),
        }
    };
}

pub(crate) use identity_scheme;
pub(crate) use identity_type;
