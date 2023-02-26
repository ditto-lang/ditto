use ditto_ast::{PrimType, Type};

pub fn mk_bool_type() -> Type {
    Type::PrimConstructor(PrimType::Bool)
}

pub fn mk_array_type(t: Type) -> Type {
    Type::Call {
        function: Box::new(Type::PrimConstructor(PrimType::Array)),
        arguments: Box::new(nonempty::nonempty![t]),
    }
}

pub fn mk_wobbly_function_type(parameters: Vec<Type>, return_type: Type) -> Type {
    super::wobbly(mk_function_type(parameters, return_type))
}

pub fn mk_function_type(parameters: Vec<Type>, return_type: Type) -> Type {
    Type::Function {
        parameters,
        return_type: Box::new(return_type),
    }
}
