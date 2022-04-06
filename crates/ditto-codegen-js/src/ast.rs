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
pub struct Block(pub Vec<BlockStatement>);

/// A single JavaScript statement.
///
/// These end with a semicolon.
pub enum BlockStatement {
    /// ```javascript
    /// const ident = expression;
    /// ```
    _ConstAssignment { ident: Ident, value: Expression },
    /// ```javascript
    /// if (condition) {
    ///     true_clause();
    /// } else {
    ///     false_clause();
    /// }
    /// ```
    If {
        condition: Expression,
        true_clause: Block,
        false_clause: Block,
    },
    /// ```javascript
    /// return bar;
    /// return;
    /// ```
    Return(Option<Expression>),
}

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
        function: Box<Expression>,
        arguments: Vec<Expression>,
    },
    /// ```javascript
    /// []
    /// [5, 5, 5]
    /// ```
    Array(Vec<Expression>),
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
}

/// The _body_ of an arrow function.
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
