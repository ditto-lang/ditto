pub struct Module {
    pub imports: Vec<ImportStatement>,
    pub statements: Vec<ModuleStatement>,
    pub exports: Vec<Ident>,
}

/// <https://developer.mozilla.org/en-US/docs/Glossary/Identifier>
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ident(pub String);

macro_rules! ident {
    ($stringy:expr) => {
        $crate::ast::Ident(String::from($stringy))
    };
}

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

/// A bunch of statements surrounded by braces.
#[derive(Clone)]
pub struct Block(pub Vec<BlockStatement>);

/// A single JavaScript statement.
///
/// These end with a semicolon.
#[derive(Clone)]
pub enum BlockStatement {
    /// ```javascript
    /// const ident = expression;
    /// ```
    ConstAssignment { ident: Ident, value: Expression },
    /// ```javascript
    /// console.log("hi");
    /// ```
    _Expression(Expression),
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

#[derive(Clone)]
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
    /// IIFE
    ///
    /// ```javascript
    /// (() => { block })()
    /// ```
    Block(Block),
    /// ```javascript
    /// 1 + 2
    /// x && y
    /// ```
    Operator {
        op: Operator,
        lhs: Box<Self>,
        rhs: Box<Self>,
    },
    IndexAccess {
        target: Box<Self>,
        index: Box<Self>,
    },
}

/// A binary operator.
#[derive(Clone)]
pub enum Operator {
    And,
    Equals,
}

/// The _body_ of an arrow function.
#[derive(Clone)]
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
