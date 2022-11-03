#![allow(dead_code)] // XXX
use crate::ast;
use egg::{Id, Language};

mod rewrites;
use rewrites::{rewrites, Rewrite};

pub enum BlockOrExpression {
    Block(ast::Block),
    Expression(ast::Expression),
}

pub fn optimize_expression(ast: ast::Expression) -> BlockOrExpression {
    optimize_expression_with(ast, &rewrites())
}

struct JavaScriptCostFn;
impl egg::CostFunction<Expression> for JavaScriptCostFn {
    type Cost = f64;
    fn cost<C>(&mut self, expr: &Expression, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let expr_cost = match expr {
            // Assigning a greater cost to call expressions should help remove IIFEs
            Expression::Call { .. } => 3.0,
            _ => 1.0,
        };
        // This otherwise looks like the egg::AstSize cost function
        // https://docs.rs/egg/0.8.1/src/egg/extract.rs.html#157
        expr.fold(expr_cost, |sum, id| sum + costs(id))
    }
}

pub(crate) fn optimize_expression_with(
    ast: ast::Expression,
    rewrites: &[Rewrite],
) -> BlockOrExpression {
    type Runner = egg::Runner<Expression, ()>;
    let mut rec_expr = RecExpr::default();
    let _id = build_rec_expr(&ast, &mut rec_expr);
    let runner = Runner::default().with_expr(&rec_expr).run(rewrites);
    let extractor = egg::Extractor::new(&runner.egraph, JavaScriptCostFn);
    let (_cost, rec_expr) = extractor.find_best(runner.roots[0]);
    let best_id = Id::from(rec_expr.as_ref().len() - 1);
    let best_expr = &rec_expr[best_id];
    unbuild_rec_expr(best_expr, &rec_expr)
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Expression {
    Conditional {
        children: [Id; 3],
    },
    Array {
        children: Vec<Id>,
    },
    Call {
        children: Vec<Id>,
    },
    ArrowFunctionBlock0 {
        children: [Id; 1],
    },
    ArrowFunctionBlock {
        children: [Id; 2],
    },
    ArrowFunctionExpr0 {
        children: [Id; 1],
    },
    ArrowFunctionExpr {
        children: [Id; 2],
    },
    ArrowFunctionParameters(Vec<ast::Ident>),
    ArrowFunctionIdentity(ast::Ident),
    IndexAccess {
        children: [Id; 2],
    },
    Operator {
        op: ast::Operator,
        children: [Id; 2],
    },
    Object {
        keys: Vec<String>,
        values: Vec<Id>,
    },
    // Blocks
    BlockExpression {
        children: [Id; 2],
    },
    BlockReturn {
        children: [Id; 1],
    },
    BlockReturnVoid,
    BlockConstAssignment {
        ident: ast::Ident,
        children: [Id; 2],
    },
    BlockIf {
        children: [Id; 3],
    },
    BlockThrow(String),
    // Leaves
    True,
    False,
    Undefined,
    Number(String),
    String(String),
    Variable(ast::Ident),
}

impl egg::Language for Expression {
    fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Conditional { .. }, Self::Conditional { .. }) => true,
            (Self::Array { children: x }, Self::Array { children: y }) => x.len() == y.len(), // REVIEW: is this length comparison right?
            (Self::Call { children: x }, Self::Call { children: y }) => x.len() == y.len(), // REVIEW: is this length comparison right?
            (Self::ArrowFunctionBlock0 { .. }, Self::ArrowFunctionBlock0 { .. }) => true,
            (Self::ArrowFunctionBlock { .. }, Self::ArrowFunctionBlock { .. }) => true,
            (Self::ArrowFunctionExpr0 { .. }, Self::ArrowFunctionExpr0 { .. }) => true,
            (Self::ArrowFunctionExpr { .. }, Self::ArrowFunctionExpr { .. }) => true,
            (Self::ArrowFunctionParameters { .. }, Self::ArrowFunctionParameters { .. }) => true,
            (Self::ArrowFunctionIdentity { .. }, Self::ArrowFunctionIdentity { .. }) => true,
            (Self::IndexAccess { .. }, Self::IndexAccess { .. }) => true,
            (Self::Operator { op: x, .. }, Self::Operator { op: y, .. }) => x == y,
            (Self::Object { .. }, Self::Object { .. }) => true, // check entries.len ?
            // Blocks
            (Self::BlockExpression { .. }, Self::BlockExpression { .. }) => true,
            (Self::BlockReturn { .. }, Self::BlockReturn { .. }) => true,
            (Self::BlockReturnVoid, Self::BlockReturnVoid) => true,
            (Self::BlockConstAssignment { .. }, Self::BlockConstAssignment { .. }) => true,
            (Self::BlockThrow { .. }, Self::BlockThrow { .. }) => true,
            (Self::BlockIf { .. }, Self::BlockIf { .. }) => true,
            // Leaves
            (Self::True, Self::True) => true,
            (Self::False, Self::False) => true,
            (Self::Undefined, Self::Undefined) => true,
            (Self::Number(x), Self::Number(y)) => *x == *y,
            (Self::String(x), Self::String(y)) => *x == *y,
            (Self::Variable(x), Self::Variable(y)) => *x == *y,
            _ => false,
        }
    }
    fn children(&self) -> &[egg::Id] {
        match self {
            Self::Conditional { children } => children,
            Self::Array { children } => children,
            Self::ArrowFunctionBlock0 { children, .. } => children,
            Self::ArrowFunctionBlock { children, .. } => children,
            Self::ArrowFunctionExpr0 { children, .. } => children,
            Self::ArrowFunctionExpr { children, .. } => children,
            Self::ArrowFunctionParameters { .. } => &[],
            Self::ArrowFunctionIdentity { .. } => &[],
            Self::Call { children } => children,
            Self::IndexAccess { children } => children,
            Self::Operator { children, .. } => children,
            Self::Object { values, .. } => values,
            // Blocks
            Self::BlockExpression { children } => children,
            Self::BlockConstAssignment { children, .. } => children,
            Self::BlockReturn { children } => children,
            Self::BlockReturnVoid => &[],
            Self::BlockThrow { .. } => &[],
            Self::BlockIf { children } => children,
            // Leaves
            Self::True => &[],
            Self::False => &[],
            Self::Undefined => &[],
            Self::Number(_) => &[],
            Self::String(_) => &[],
            Self::Variable(_) => &[],
        }
    }
    fn children_mut(&mut self) -> &mut [egg::Id] {
        match self {
            Self::Conditional { children } => children,
            Self::Array { children } => children,
            Self::ArrowFunctionBlock0 { children, .. } => children,
            Self::ArrowFunctionBlock { children, .. } => children,
            Self::ArrowFunctionExpr0 { children, .. } => children,
            Self::ArrowFunctionExpr { children, .. } => children,
            Self::ArrowFunctionParameters { .. } => &mut [],
            Self::ArrowFunctionIdentity { .. } => &mut [],
            Self::Call { children } => children,
            Self::IndexAccess { children } => children,
            Self::Operator { children, .. } => children,
            Self::Object { values, .. } => values,
            // Blocks
            Self::BlockConstAssignment { children, .. } => children,
            Self::BlockExpression { children } => children,
            Self::BlockReturn { children } => children,
            Self::BlockReturnVoid => &mut [],
            Self::BlockThrow { .. } => &mut [],
            Self::BlockIf { children } => children,
            // Leaves
            Self::True => &mut [],
            Self::False => &mut [],
            Self::Undefined => &mut [],
            Self::Number(_) => &mut [],
            Self::String(_) => &mut [],
            Self::Variable(_) => &mut [],
        }
    }
}

type RecExpr = egg::RecExpr<Expression>;

fn build_rec_expr(ast_expr: &ast::Expression, rec_expr: &mut RecExpr) -> Id {
    ast_expr_to_rec_expr(ast_expr, rec_expr)
}

fn ast_expr_to_rec_expr(ast_expr: &ast::Expression, rec_expr: &mut RecExpr) -> Id {
    match ast_expr {
        ast::Expression::Conditional {
            condition,
            true_clause,
            false_clause,
        } => {
            let children = [
                ast_expr_to_rec_expr(condition, rec_expr),
                ast_expr_to_rec_expr(true_clause, rec_expr),
                ast_expr_to_rec_expr(false_clause, rec_expr),
            ];
            let node = Expression::Conditional { children };
            rec_expr.add(node)
        }
        ast::Expression::Array(elements) => {
            let children = elements
                .iter()
                .map(|element| ast_expr_to_rec_expr(element, rec_expr))
                .collect();
            let node = Expression::Array { children };
            rec_expr.add(node)
        }
        ast::Expression::ArrowFunction {
            parameters,
            body: box ast::ArrowFunctionBody::Block(body),
        } => match parameters.as_slice() {
            [] => {
                let body = ast_block_to_rec_expr(body, rec_expr);
                let children = [body];
                let node = Expression::ArrowFunctionBlock0 { children };
                rec_expr.add(node)
            }
            _ => {
                let parameters =
                    rec_expr.add(Expression::ArrowFunctionParameters(parameters.to_vec()));
                let body = ast_block_to_rec_expr(body, rec_expr);
                let children = [parameters, body];
                let node = Expression::ArrowFunctionBlock { children };
                rec_expr.add(node)
            }
        },
        ast::Expression::ArrowFunction {
            parameters,
            body: box ast::ArrowFunctionBody::Expression(body),
        } => match (parameters.as_slice(), body) {
            ([], _) => {
                let body = ast_expr_to_rec_expr(body, rec_expr);
                let children = [body];
                let node = Expression::ArrowFunctionExpr0 { children };
                rec_expr.add(node)
            }
            ([ident_param], ast::Expression::Variable(ident_body)) if ident_param == ident_body => {
                let node = Expression::ArrowFunctionIdentity(ident_param.clone());
                rec_expr.add(node)
            }
            _ => {
                let parameters =
                    rec_expr.add(Expression::ArrowFunctionParameters(parameters.to_vec()));
                let body = ast_expr_to_rec_expr(body, rec_expr);
                let children = [parameters, body];
                let node = Expression::ArrowFunctionExpr { children };
                rec_expr.add(node)
            }
        },
        ast::Expression::Call {
            function,
            arguments,
        } => {
            let mut children = vec![ast_expr_to_rec_expr(function, rec_expr)];
            children.extend(
                arguments
                    .iter()
                    .map(|arg| ast_expr_to_rec_expr(arg, rec_expr)),
            );
            let node = Expression::Call { children };
            rec_expr.add(node)
        }
        ast::Expression::IndexAccess { target, index } => {
            let children = [
                ast_expr_to_rec_expr(target, rec_expr),
                ast_expr_to_rec_expr(index, rec_expr),
            ];
            let node = Expression::IndexAccess { children };
            rec_expr.add(node)
        }
        ast::Expression::Operator { op, lhs, rhs } => {
            let children = [
                ast_expr_to_rec_expr(lhs, rec_expr),
                ast_expr_to_rec_expr(rhs, rec_expr),
            ];
            let node = Expression::Operator {
                op: op.clone(),
                children,
            };
            rec_expr.add(node)
        }
        ast::Expression::Object(entries) => {
            let mut keys = Vec::with_capacity(entries.len());
            let mut values = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                keys.push(key.clone());
                values.push(ast_expr_to_rec_expr(value, rec_expr));
            }
            let node = Expression::Object { keys, values };
            rec_expr.add(node)
        }
        // Leaves
        ast::Expression::True => rec_expr.add(Expression::True),
        ast::Expression::False => rec_expr.add(Expression::False),
        ast::Expression::Undefined => rec_expr.add(Expression::Undefined),
        ast::Expression::Number(number) => rec_expr.add(Expression::Number(number.clone())),
        ast::Expression::String(string) => rec_expr.add(Expression::String(string.clone())),
        ast::Expression::Variable(ident) => rec_expr.add(Expression::Variable(ident.clone())),
    }
}

fn ast_block_to_rec_expr(ast_block: &ast::Block, rec_expr: &mut RecExpr) -> Id {
    match ast_block {
        ast::Block::Return(Some(ast_expr)) => {
            let children = [ast_expr_to_rec_expr(ast_expr, rec_expr)];
            let node = Expression::BlockReturn { children };
            rec_expr.add(node)
        }
        ast::Block::Return(None) => {
            let node = Expression::BlockReturnVoid;
            rec_expr.add(node)
        }
        ast::Block::Expression {
            expression: ast_expr,
            rest,
        } => {
            let children = [
                ast_expr_to_rec_expr(ast_expr, rec_expr),
                ast_block_to_rec_expr(rest, rec_expr),
            ];
            let node = Expression::BlockExpression { children };
            rec_expr.add(node)
        }
        ast::Block::Throw(msg) => {
            let node = Expression::BlockThrow(msg.to_string());
            rec_expr.add(node)
        }
        ast::Block::ConstAssignment { ident, value, rest } => {
            let children = [
                ast_expr_to_rec_expr(value, rec_expr),
                ast_block_to_rec_expr(rest, rec_expr),
            ];
            let node = Expression::BlockConstAssignment {
                ident: ident.clone(),
                children,
            };
            rec_expr.add(node)
        }
        ast::Block::If {
            condition,
            true_branch,
            false_branch,
        } => {
            let children = [
                ast_expr_to_rec_expr(condition, rec_expr),
                ast_block_to_rec_expr(true_branch, rec_expr),
                ast_block_to_rec_expr(false_branch, rec_expr),
            ];
            let node = Expression::BlockIf { children };
            rec_expr.add(node)
        }
    }
}

// TODO: consider returning a `Result` here so we can fail gracefully?
// but hopefully it shouldn't fail...

fn unbuild_rec_expr(expr: &Expression, rec_expr: &RecExpr) -> BlockOrExpression {
    match expr {
        Expression::BlockExpression { .. }
        | Expression::BlockReturn { .. }
        | Expression::BlockThrow { .. }
        | Expression::BlockReturnVoid
        | Expression::BlockConstAssignment { .. }
        | Expression::BlockIf { .. } => {
            BlockOrExpression::Block(rec_expr_to_ast_block(expr, rec_expr))
        }
        Expression::Conditional { .. }
        | Expression::Array { .. }
        | Expression::Call { .. }
        | Expression::ArrowFunctionBlock0 { .. }
        | Expression::ArrowFunctionBlock { .. }
        | Expression::ArrowFunctionExpr0 { .. }
        | Expression::ArrowFunctionExpr { .. }
        | Expression::ArrowFunctionIdentity { .. }
        | Expression::IndexAccess { .. }
        | Expression::Operator { .. }
        | Expression::Object { .. }
        | Expression::True
        | Expression::False
        | Expression::Undefined
        | Expression::Number { .. }
        | Expression::String { .. }
        | Expression::Variable { .. } => {
            BlockOrExpression::Expression(rec_expr_to_ast_expr(expr, rec_expr))
        }
        Expression::ArrowFunctionParameters { .. } => unreachable!(),
    }
}

fn rec_expr_to_ast_expr(expr: &Expression, rec_expr: &RecExpr) -> ast::Expression {
    match expr {
        Expression::Conditional {
            children: [condition_id, true_clause_id, false_clause_id],
        } => ast::Expression::Conditional {
            condition: Box::new(rec_expr_to_ast_expr(&rec_expr[*condition_id], rec_expr)),
            true_clause: Box::new(rec_expr_to_ast_expr(&rec_expr[*true_clause_id], rec_expr)),
            false_clause: Box::new(rec_expr_to_ast_expr(&rec_expr[*false_clause_id], rec_expr)),
        },
        Expression::Array { children } => ast::Expression::Array(
            children
                .iter()
                .map(|id| rec_expr_to_ast_expr(&rec_expr[*id], rec_expr))
                .collect(),
        ),
        Expression::Call { children } => {
            let (function_id, argument_ids) = children.split_first().unwrap();
            let function = Box::new(rec_expr_to_ast_expr(&rec_expr[*function_id], rec_expr));
            let arguments = argument_ids
                .iter()
                .map(|arg_id| rec_expr_to_ast_expr(&rec_expr[*arg_id], rec_expr))
                .collect();
            ast::Expression::Call {
                function,
                arguments,
            }
        }
        Expression::ArrowFunctionBlock0 {
            children: [body_id],
        } => {
            let body = rec_expr_to_ast_block(&rec_expr[*body_id], rec_expr);
            ast::Expression::ArrowFunction {
                parameters: vec![],
                body: Box::new(ast::ArrowFunctionBody::Block(body)),
            }
        }
        Expression::ArrowFunctionBlock {
            children: [parameters_id, body_id],
        } => {
            if let Expression::ArrowFunctionParameters(parameters) = &rec_expr[*parameters_id] {
                let body = rec_expr_to_ast_block(&rec_expr[*body_id], rec_expr);
                ast::Expression::ArrowFunction {
                    parameters: parameters.to_vec(),
                    body: Box::new(ast::ArrowFunctionBody::Block(body)),
                }
            } else {
                unreachable!()
            }
        }
        Expression::ArrowFunctionExpr0 {
            children: [body_id],
        } => {
            let body = rec_expr_to_ast_expr(&rec_expr[*body_id], rec_expr);
            ast::Expression::ArrowFunction {
                parameters: vec![],
                body: Box::new(ast::ArrowFunctionBody::Expression(body)),
            }
        }
        Expression::ArrowFunctionExpr {
            children: [parameters_id, body_id],
        } => {
            if let Expression::ArrowFunctionParameters(parameters) = &rec_expr[*parameters_id] {
                let body = rec_expr_to_ast_expr(&rec_expr[*body_id], rec_expr);
                ast::Expression::ArrowFunction {
                    parameters: parameters.to_vec(),
                    body: Box::new(ast::ArrowFunctionBody::Expression(body)),
                }
            } else {
                unreachable!()
            }
        }
        Expression::ArrowFunctionIdentity(ident) => ast::Expression::ArrowFunction {
            parameters: vec![ident.clone()],
            body: Box::new(ast::ArrowFunctionBody::Expression(
                ast::Expression::Variable(ident.clone()),
            )),
        },
        Expression::IndexAccess {
            children: [target_id, index_id],
        } => ast::Expression::IndexAccess {
            target: Box::new(rec_expr_to_ast_expr(&rec_expr[*target_id], rec_expr)),
            index: Box::new(rec_expr_to_ast_expr(&rec_expr[*index_id], rec_expr)),
        },
        Expression::Operator {
            op,
            children: [lhs_id, rhs_id],
        } => ast::Expression::Operator {
            op: op.clone(),
            lhs: Box::new(rec_expr_to_ast_expr(&rec_expr[*lhs_id], rec_expr)),
            rhs: Box::new(rec_expr_to_ast_expr(&rec_expr[*rhs_id], rec_expr)),
        },
        Expression::Object { keys, values } => {
            let mut entries = indexmap::IndexMap::with_capacity(keys.len());
            for (key, value_id) in keys.iter().zip(values) {
                let value = rec_expr_to_ast_expr(&rec_expr[*value_id], rec_expr);
                entries.insert(key.clone(), value);
            }
            ast::Expression::Object(entries)
        }
        // Leaves
        Expression::True => ast::Expression::True,
        Expression::False => ast::Expression::False,
        Expression::Undefined => ast::Expression::Undefined,
        Expression::Number(number) => ast::Expression::Number(number.clone()),
        Expression::String(string) => ast::Expression::String(string.clone()),
        Expression::Variable(ident) => ast::Expression::Variable(ident.clone()),

        // wut
        Expression::BlockExpression { .. }
        | Expression::BlockReturn { .. }
        | Expression::BlockThrow { .. }
        | Expression::BlockReturnVoid
        | Expression::BlockConstAssignment { .. }
        | Expression::BlockIf { .. } => {
            unreachable!();
        }
        Expression::ArrowFunctionParameters { .. } => {
            unreachable!();
        }
    }
}

fn rec_expr_to_ast_block(expr: &Expression, rec_expr: &RecExpr) -> ast::Block {
    match expr {
        Expression::BlockReturn {
            children: [expression_id],
        } => {
            let expression = rec_expr_to_ast_expr(&rec_expr[*expression_id], rec_expr);
            ast::Block::Return(Some(expression))
        }
        Expression::BlockReturnVoid => ast::Block::Return(None),
        Expression::BlockConstAssignment {
            ident,
            children: [value_id, rest_id],
        } => {
            let value = rec_expr_to_ast_expr(&rec_expr[*value_id], rec_expr);
            let rest = Box::new(rec_expr_to_ast_block(&rec_expr[*rest_id], rec_expr));
            ast::Block::ConstAssignment {
                ident: ident.clone(),
                value,
                rest,
            }
        }
        Expression::BlockExpression {
            children: [expression_id, rest_id],
        } => {
            let expression = rec_expr_to_ast_expr(&rec_expr[*expression_id], rec_expr);
            let rest = Box::new(rec_expr_to_ast_block(&rec_expr[*rest_id], rec_expr));
            ast::Block::Expression { expression, rest }
        }
        Expression::BlockThrow(msg) => ast::Block::Throw(msg.to_string()),
        Expression::BlockIf {
            children: [condition_id, true_branch_id, false_branch_id],
        } => {
            let condition = rec_expr_to_ast_expr(&rec_expr[*condition_id], rec_expr);
            let true_branch = Box::new(rec_expr_to_ast_block(&rec_expr[*true_branch_id], rec_expr));
            let false_branch =
                Box::new(rec_expr_to_ast_block(&rec_expr[*false_branch_id], rec_expr));
            ast::Block::If {
                condition,
                true_branch,
                false_branch,
            }
        }

        // wut
        Expression::Conditional { .. }
        | Expression::Array { .. }
        | Expression::Call { .. }
        | Expression::ArrowFunctionBlock0 { .. }
        | Expression::ArrowFunctionBlock { .. }
        | Expression::ArrowFunctionExpr0 { .. }
        | Expression::ArrowFunctionExpr { .. }
        | Expression::ArrowFunctionIdentity { .. }
        | Expression::IndexAccess { .. }
        | Expression::Operator { .. }
        | Expression::Object { .. }
        | Expression::True
        | Expression::False
        | Expression::Undefined
        | Expression::Number { .. }
        | Expression::String { .. }
        | Expression::Variable { .. } => {
            unreachable!();
        }
        // rlly wut
        Expression::ArrowFunctionParameters { .. } => {
            unreachable!();
        }
    }
}

#[cfg(test)]
mod test {
    use super::{optimize_expression_with, test_macros::assert_optimized, BlockOrExpression};
    use crate::ast;
    use quickcheck::{quickcheck, Arbitrary, Gen};

    #[test]
    fn roundtrippin() {
        assert_optimized!("fn (x, _y) -> x", "(x,_y) => x", &[]);
        assert_optimized!("fn (x, _y) -> if x then 5 else 10", "(x,_y) => x?5:10", &[]);
        assert_optimized!(
            "fn (x, y) -> if x then 5 else if y then 10 else 15",
            "(x,y) => x?5:y?10:15",
            &[]
        );
        assert_optimized!(
            "fn (a) -> match a with | A -> 5 end",
            "(a) => (a[0] === \"A\")?5:(() => {throw new Error(\"Pattern match error\");})()",
            &[]
        );
        assert_optimized!(
            "fn (bc) -> match bc with | B -> 1 | C -> 2 end",
            "(bc) => (bc[0] === \"B\")?1:(bc[0] === \"C\")?2:(() => {throw new Error(\"Pattern match error\");})()",
            &[]
        );
    }

    // Test that we can roundtrip between the expression representations!
    quickcheck! {
        fn prop_roundtrip(want: ast::Expression) -> bool {
            let roundtripped = optimize_expression_with(want.clone(), &[]);
            match roundtripped {
                BlockOrExpression::Block(_) => panic!("Expression went in, Block came out!?"),
                BlockOrExpression::Expression(got) => want == got
            }
        }
    }

    impl Arbitrary for ast::Expression {
        fn arbitrary(g: &mut Gen) -> Self {
            Self::arbitrary_stacksafe(g, 5)
        }
    }

    impl ast::Expression {
        fn arbitrary_stacksafe(g: &mut Gen, depth: usize) -> Self {
            if depth == 0 {
                // Generate leaf
                let variable = Self::Variable(ast::Ident::arbitrary(g));
                let number = Self::Number(String::arbitrary(g));
                let string = Self::String(String::arbitrary(g));
                g.choose(&[
                    Self::True,
                    Self::False,
                    Self::Undefined,
                    number,
                    string,
                    variable,
                ])
                .cloned()
                .unwrap()
            } else {
                // Laziness hack :(
                let i = g
                    .choose(&[
                        0, // ArrowFunction
                        1, // Call
                        2, // Conditional
                        3, // Array
                        4, // Operator
                        5, // IndexAccess
                        6, // IndexAccess
                    ])
                    .cloned()
                    .unwrap();
                match i {
                    0 => Self::ArrowFunction {
                        parameters: vec![
                            ast::Ident::arbitrary(g),
                            ast::Ident::arbitrary(g),
                            ast::Ident::arbitrary(g),
                        ],
                        body: Box::new(ast::ArrowFunctionBody::arbitrary_stacksafe(g, depth - 1)),
                    },
                    1 => Self::Call {
                        function: Box::new(Self::arbitrary_stacksafe(g, depth - 1)),
                        arguments: vec![
                            Self::arbitrary_stacksafe(g, depth - 1),
                            Self::arbitrary_stacksafe(g, depth - 1),
                            Self::arbitrary_stacksafe(g, depth - 1),
                        ],
                    },
                    2 => Self::Conditional {
                        condition: Box::new(Self::arbitrary_stacksafe(g, depth - 1)),
                        true_clause: Box::new(Self::arbitrary_stacksafe(g, depth - 1)),
                        false_clause: Box::new(Self::arbitrary_stacksafe(g, depth - 1)),
                    },
                    3 => Self::Array(vec![
                        Self::arbitrary_stacksafe(g, depth - 1),
                        Self::arbitrary_stacksafe(g, depth - 1),
                        Self::arbitrary_stacksafe(g, depth - 1),
                        Self::arbitrary_stacksafe(g, depth - 1),
                    ]),
                    4 => {
                        let op = g
                            .choose(&[ast::Operator::And, ast::Operator::Equals])
                            .cloned()
                            .unwrap();
                        Self::Operator {
                            op,
                            lhs: Box::new(Self::arbitrary_stacksafe(g, depth - 1)),
                            rhs: Box::new(Self::arbitrary_stacksafe(g, depth - 1)),
                        }
                    }
                    5 => Self::IndexAccess {
                        target: Box::new(Self::arbitrary_stacksafe(g, depth - 1)),
                        index: Box::new(Self::arbitrary_stacksafe(g, depth - 1)),
                    },
                    6 => Self::Object({
                        let mut entries = indexmap::IndexMap::with_capacity(3);
                        entries.insert(
                            String::arbitrary(g),
                            Self::arbitrary_stacksafe(g, depth - 1),
                        );
                        entries.insert(
                            String::arbitrary(g),
                            Self::arbitrary_stacksafe(g, depth - 1),
                        );
                        entries.insert(
                            String::arbitrary(g),
                            Self::arbitrary_stacksafe(g, depth - 1),
                        );
                        entries
                    }),
                    _ => unreachable!(),
                }
            }
        }
    }

    impl ast::Block {
        fn arbitrary_stacksafe(g: &mut Gen, depth: usize) -> Self {
            // Generate leaf
            if depth == 0 {
                let return_expr = Self::Return(Some(ast::Expression::arbitrary_stacksafe(g, 0)));
                let throw = Self::Throw(String::arbitrary(g));
                g.choose(&[throw, return_expr, Self::Return(None)])
                    .cloned()
                    .unwrap()
            } else {
                // Laziness hack :(
                let i = g
                    .choose(&[
                        0, // ConstAssignment
                        1, // Expression
                        2, // If
                    ])
                    .cloned()
                    .unwrap();
                match i {
                    0 => Self::ConstAssignment {
                        ident: ast::Ident::arbitrary(g),
                        value: ast::Expression::arbitrary_stacksafe(g, depth - 1),
                        rest: Box::new(Self::arbitrary_stacksafe(g, depth - 1)),
                    },
                    1 => Self::Expression {
                        expression: ast::Expression::arbitrary_stacksafe(g, depth - 1),
                        rest: Box::new(Self::arbitrary_stacksafe(g, depth - 1)),
                    },
                    2 => Self::If {
                        condition: ast::Expression::arbitrary_stacksafe(g, depth - 1),
                        true_branch: Box::new(Self::arbitrary_stacksafe(g, depth - 1)),
                        false_branch: Box::new(Self::arbitrary_stacksafe(g, depth - 1)),
                    },
                    _ => unreachable!(),
                }
            }
        }
    }

    impl ast::ArrowFunctionBody {
        fn arbitrary_stacksafe(g: &mut Gen, depth: usize) -> Self {
            let expression_body = Self::Expression(ast::Expression::arbitrary_stacksafe(g, depth));
            let block_body = Self::Block(ast::Block::arbitrary_stacksafe(g, depth));
            g.choose(&[expression_body, block_body]).cloned().unwrap()
        }
    }

    impl Arbitrary for ast::Ident {
        fn arbitrary(g: &mut Gen) -> Self {
            Self(String::arbitrary(g))
        }
    }
}

#[cfg(test)]
mod test_macros {
    macro_rules! assert_optimized {
        ($source:expr, $want:expr) => {{
            $crate::optimize::test_macros::assert_optimized!(
                $source,
                $want,
                &$crate::optimize::rewrites::rewrites()
            );
        }};
        ($source:expr, $want:expr, $rewrites:expr) => {{
            let cst_module = ditto_cst::Module::parse(&format!(
                "module Test exports (..); type A = A; type BC = B | C; expr = {};",
                $source
            ))
            .unwrap();
            let (ast_module, _warnings) =
                ditto_checker::check_module(&ditto_checker::Everything::default(), cst_module)
                    .unwrap();

            let ast_expression = ast_module.values.into_values().next().unwrap().expression;
            let js_expression = $crate::convert::convert_expression(
                &mut $crate::convert::Supply::default(),
                &mut $crate::convert::ImportedModuleIdents::new(),
                ast_expression,
            );

            // Debug the unoptimised expression for debuggin'
            // (it will be printed if the test fails)
            let mut unoptimised = String::new();
            $crate::render::Render::render(&js_expression, &mut unoptimised);
            dbg!(unoptimised);

            let block_or_expression =
                $crate::optimize::optimize_expression_with(js_expression, $rewrites);
            let mut rendered = String::new();
            match block_or_expression {
                $crate::optimize::BlockOrExpression::Block(block) => {
                    $crate::render::Render::render(&block, &mut rendered);
                }
                $crate::optimize::BlockOrExpression::Expression(expr) => {
                    $crate::render::Render::render(&expr, &mut rendered);
                }
            }
            similar_asserts::assert_eq!(got: rendered, want: $want);
        }};
    }

    pub(crate) use assert_optimized;
}
