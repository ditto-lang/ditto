use ditto_ast::{FullyQualifiedProperName, ProperName, QualifiedProperName, Type};
use halfbrown::HashMap;

pub type EnvConstructors = HashMap<QualifiedProperName, EnvConstructor>;

#[derive(Clone)]
pub enum EnvConstructor {
    ModuleConstructor {
        constructor: ProperName,
        constructor_type: Type,
    },
    ImportedConstructor {
        constructor: FullyQualifiedProperName,
        constructor_type: Type,
    },
}

impl EnvConstructor {
    pub fn get_type(&self) -> &Type {
        match self {
            Self::ModuleConstructor {
                constructor_type, ..
            } => constructor_type,
            Self::ImportedConstructor {
                constructor_type, ..
            } => constructor_type,
        }
    }

    pub fn get_terminal_type(&self) -> &Type {
        match self.get_type() {
            Type::Function {
                box return_type, ..
            } => {
                // Type constructors aren't curried!
                return_type
            }
            // This should either be a type constructor or a call of a type constructor!
            other => other,
        }
    }
}
