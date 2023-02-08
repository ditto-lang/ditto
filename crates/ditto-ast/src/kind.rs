use bincode::{Decode, Encode};
use non_empty_vec::NonEmpty;
use serde::{Deserialize, Serialize};

/// The kind of types.
///
/// Note that there is currently no source representation for kinds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub enum Kind {
    /// Also known as `*` to functional programming folk.
    Type,
    /// A kind variable.
    Variable(usize),
    /// The kind of types that need to be applied to other types.
    ///
    /// For example: the kind of `Array` is `(Kind::Type) -> Kind::Type`
    ///
    /// Note that nullary types (e.g. `Foo()`) are not allowed.
    /// hence the non-empty-ness.
    ///
    /// Also note that the "return kind" can only be `Kind::Type` at the moment.
    Function {
        /// The kinds of the arguments this type expects.
        #[bincode(with_serde)]
        parameters: NonEmpty<Self>,
    },
    /// A series of labelled types. Used for records.
    Row,
}

impl Kind {
    /// Render the kind as a compact, single-line string.
    /// Useful for testing and debugging, but not much else...
    pub fn debug_render(&self) -> String {
        self.debug_render_with(|var| format!("${}", var))
    }

    /// Render the kind as a compact, single-line string.
    /// Useful for testing and debugging, but not much else...
    ///
    /// The caller must decide how to render kind variables via `render_var`.
    pub fn debug_render_with<F>(&self, render_var: F) -> String
    where
        F: Fn(usize) -> String + Copy,
    {
        match self {
            Self::Variable(var) => render_var(*var),
            Self::Type => String::from("Type"),
            Self::Function { parameters } => {
                let mut output = String::from("(");
                let len = parameters.len();
                parameters.iter().enumerate().for_each(|(i, param)| {
                    output.push_str(&param.debug_render());
                    if i + 1 != len.into() {
                        output.push_str(", ");
                    }
                });
                output.push_str(") -> Type");
                output
            }
            Self::Row => String::from("Row"),
        }
    }
}
