pub struct Module {
    pub imports: Vec<ImportStatement>,
    pub statements: Vec<ModuleStatement>,
    pub exports: Vec<Ident>,
}

/// <https://developer.mozilla.org/en-US/docs/Glossary/Identifier>
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ident(pub String);

#[cfg(test)]
macro_rules! ident {
    ($stringy:expr) => {
        $crate::ast::Ident(String::from($stringy))
    };
}

#[cfg(test)]
pub(crate) use ident;

pub struct ImportStatement {
    pub idents: Vec<(Ident, Ident)>,
    //               foo as bar
    pub path: String,
}

pub enum ModuleStatement {
    /// ```javascript
    /// const ident = expression
    /// ```
    ConstAssignment { ident: Ident, value: Expression },
    /// ```javascript
    /// ident = expression
    /// ```
    Assignment { ident: Ident, value: Expression },
    /// ```javascript
    /// let ident;
    /// ```
    LetDeclaration { ident: Ident },
    /// ```javascript
    /// function ident(parameter, parameter) { body }
    /// ```
    Function {
        ident: Ident,
        parameters: Vec<Ident>,
        body: Block,
    },
}

impl ModuleStatement {
    /// Get the identifier for this module statement.
    pub fn ident(&self) -> &Ident {
        match self {
            Self::ConstAssignment { ident, .. } => ident,
            Self::Assignment { ident, .. } => ident,
            Self::LetDeclaration { ident, .. } => ident,
            Self::Function { ident, .. } => ident,
        }
    }
}

/// A bunch of statements surrounded by braces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    /// ```javascript
    /// const ident = expression;
    /// ```
    ConstAssignment {
        ident: Ident,
        value: Expression,
        rest: Box<Self>,
    },

    /// ```javascript
    /// if (condition) { true_branch } false_branch
    /// ```
    If {
        condition: Expression,
        true_branch: Box<Self>,
        false_branch: Box<Self>,
    },
    /// ```javascript
    /// console.log("hi");
    /// ```
    Expression {
        expression: Expression,
        rest: Box<Self>,
    },
    /// ```javascript
    /// throw new Error("message")
    /// ```
    Throw(String),
    /// ```javascript
    /// return bar;
    /// return;
    /// ```
    Return(Option<Expression>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expression {
    /// `true`
    True,
    /// `false`
    False,
    /// ```javascript
    /// foo
    /// Bar
    /// $baz
    ///
    /// ```
    Variable(Ident),
    /// ```javascript
    /// (parameter, parameter) => { body }
    /// ```
    ArrowFunction {
        parameters: Vec<Ident>,
        body: Box<ArrowFunctionBody>,
    },
    /// ```javascript
    /// function(argument, argument, argument)
    /// ```
    Call {
        function: Box<Self>,
        arguments: Vec<Self>,
    },
    /// ```javascript
    /// condition ? true_clause : false_clause
    /// ```
    Conditional {
        condition: Box<Self>,
        true_clause: Box<Self>,
        false_clause: Box<Self>,
    },
    /// ```javascript
    /// []
    /// [5, 5, 5]
    /// ```
    Array(Vec<Self>),
    /// ```javascript
    /// 5
    /// 5.0
    /// ```
    Number(String),
    /// ```javascript
    /// "five"
    /// ```
    String(String),
    /// ```javascript
    /// undefined
    /// ```
    Undefined,
    /// ```javascript
    /// 1 + 2
    /// x && y
    /// ```
    Operator {
        op: Operator,
        lhs: Box<Self>,
        rhs: Box<Self>,
    },
    /// ```javascript
    /// target[index]
    /// ```
    IndexAccess { target: Box<Self>, index: Box<Self> },
    /// ```javascript
    /// { "key": value }
    /// { ...expr, "key": value }
    /// ```
    Object {
        spread: Option<Box<Self>>,
        entries: indexmap::IndexMap<String, Self>,
    },
}

/// A binary operator.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Operator {
    And,
    Equals,
}

/// The _body_ of an arrow function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArrowFunctionBody {
    /// ```javascript
    /// () => expression;
    /// ```
    Expression(Expression),
    /// ```javascript
    /// () => { block }
    /// ```
    Block(Block),
}

/// IIFE
///
/// ```javascript
/// (() => { block })()
/// ```
macro_rules! iife {
    ($block: expr) => {
        $crate::ast::Expression::Call {
            function: Box::new($crate::ast::Expression::ArrowFunction {
                parameters: vec![],
                body: Box::new($crate::ast::ArrowFunctionBody::Block($block)),
            }),
            arguments: vec![],
        }
    };
}
pub(crate) use iife;
