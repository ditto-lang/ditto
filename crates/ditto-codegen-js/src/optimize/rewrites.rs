use super::Expression;
use crate::ast::Ident;
use core::str::FromStr;
use egg::{ENodeOrVar, Var};

pub type Rewrite = egg::Rewrite<Expression, ()>;

pub fn rewrites() -> [Rewrite; 13] {
    [
        rewrite_ternary_with_static_true_condition(),
        rewrite_ternary_with_static_false_condition(),
        rewrite_redundant_ternary(),
        rewrite_arrow_expr_iife_to_arrow_block(),
        rewrite_ternary_with_iife_true_clause_to_block(),
        rewrite_ternary_with_iife_false_clause_to_block(),
        rewrite_inline_returned_iife_block(),
        rewrite_redundant_return_undefined(),
        rewrite_inline_immediate_identity_call(),
        rewrite_ternary_call0(), // TODO: introduce `CallParams` variant and handle n-ary ternary calls
        rewrite_redundant_arrow0_block(),
        rewrite_redundant_arrow_block(),
        rewrite_redundant_iife(),
    ]
}

// If the condition of a ternary operator is `true` then we can just inline the true clause.
//
//      true ? yeh : nah
//      👉 yeh
//
fn rewrite_ternary_with_static_true_condition() -> Rewrite {
    let true_clause_var = Var::from_str("?true").unwrap();
    let false_clause_var = Var::from_str("?false").unwrap();

    let mut searcher = egg::RecExpr::default();
    let condition_id = searcher.add(ENodeOrVar::ENode(Expression::True));
    let true_clause_id = searcher.add(ENodeOrVar::Var(true_clause_var));
    let false_clause_id = searcher.add(ENodeOrVar::Var(false_clause_var));
    searcher.add(ENodeOrVar::ENode(Expression::Conditional {
        children: [condition_id, true_clause_id, false_clause_id],
    }));

    let mut applier = egg::RecExpr::default();
    applier.add(searcher[true_clause_id].clone());

    Rewrite::new(
        egg::Symbol::from("ternary_with_static_true_condition"),
        egg::Pattern::new(searcher),
        egg::Pattern::new(applier),
    )
    .unwrap()
}

// If the condition of a ternary operator is `false` then we can just inline the false clause.
//
//      false ? yeh : nah
//      👉 nah
//
fn rewrite_ternary_with_static_false_condition() -> Rewrite {
    let true_clause_var = Var::from_str("?true").unwrap();
    let false_clause_var = Var::from_str("?false").unwrap();

    let mut searcher = egg::RecExpr::default();
    let condition_id = searcher.add(ENodeOrVar::ENode(Expression::False));
    let true_clause_id = searcher.add(ENodeOrVar::Var(true_clause_var));
    let false_clause_id = searcher.add(ENodeOrVar::Var(false_clause_var));
    searcher.add(ENodeOrVar::ENode(Expression::Conditional {
        children: [condition_id, true_clause_id, false_clause_id],
    }));

    let mut applier = egg::RecExpr::default();
    applier.add(searcher[false_clause_id].clone());

    Rewrite::new(
        egg::Symbol::from("ternary_with_static_false_condition"),
        egg::Pattern::new(searcher),
        egg::Pattern::new(applier),
    )
    .unwrap()
}

// If a ternary returns a boolean, then we can just inline the condition.
//
//      condition ? true : false
//      👉 condition
//
fn rewrite_redundant_ternary() -> Rewrite {
    let condition_var = Var::from_str("?condition").unwrap();

    let mut searcher = egg::RecExpr::default();
    let condition_id = searcher.add(ENodeOrVar::Var(condition_var));
    let true_clause_id = searcher.add(ENodeOrVar::ENode(Expression::True));
    let false_clause_id = searcher.add(ENodeOrVar::ENode(Expression::False));
    searcher.add(ENodeOrVar::ENode(Expression::Conditional {
        children: [condition_id, true_clause_id, false_clause_id],
    }));

    let mut applier = egg::RecExpr::default();
    applier.add(searcher[condition_id].clone());

    Rewrite::new(
        egg::Symbol::from("redundant_ternary"),
        egg::Pattern::new(searcher),
        egg::Pattern::new(applier),
    )
    .unwrap()
}

// An arrow function body that is an IIFE can be rewritten to be a block.
//
//      (...args) => (() => { block })()
//      👉 (...args) => { block }
//
fn rewrite_arrow_expr_iife_to_arrow_block() -> Rewrite {
    let params_var = Var::from_str("?params").unwrap();
    let block_var = Var::from_str("?block").unwrap();

    let mut searcher = egg::RecExpr::default();
    let block_id = searcher.add(ENodeOrVar::Var(block_var));
    let arrow_function_id = searcher.add(ENodeOrVar::ENode(Expression::ArrowFunctionBlock0 {
        children: [block_id],
    }));
    let iife_id = searcher.add(ENodeOrVar::ENode(Expression::Call {
        children: vec![arrow_function_id], // no arguments
    }));
    let params_id = searcher.add(ENodeOrVar::Var(params_var));
    searcher.add(ENodeOrVar::ENode(Expression::ArrowFunctionExpr {
        children: [params_id, iife_id],
    }));

    let mut applier = egg::RecExpr::default();
    let params_id = applier.add(searcher[params_id].clone());
    let block_id = applier.add(searcher[block_id].clone());
    applier.add(ENodeOrVar::ENode(Expression::ArrowFunctionBlock {
        children: [params_id, block_id],
    }));

    Rewrite::new(
        egg::Symbol::from("arrow_expr_iife_to_arrow_block"),
        egg::Pattern::new(searcher),
        egg::Pattern::new(applier),
    )
    .unwrap()
}

// A ternary with an IIFE true clause can be rewritten as an IIFE containing an `if` block.
//
//      condition ? (() => { block })() : 5
//      👉 (() => { if (condition) { block } return 5 })()
//
fn rewrite_ternary_with_iife_true_clause_to_block() -> Rewrite {
    let condition_var = Var::from_str("?condition").unwrap();
    let block_var = Var::from_str("?block").unwrap();
    let false_clause_var = Var::from_str("?false_clause").unwrap();

    let mut searcher = egg::RecExpr::default();
    let condition_id = searcher.add(ENodeOrVar::Var(condition_var));
    let block_id = searcher.add(ENodeOrVar::Var(block_var));
    let arrow_function_id = searcher.add(ENodeOrVar::ENode(Expression::ArrowFunctionBlock0 {
        children: [block_id],
    }));
    let true_clause_id = searcher.add(ENodeOrVar::ENode(Expression::Call {
        children: vec![arrow_function_id], // no arguments
    }));
    let false_clause_id = searcher.add(ENodeOrVar::Var(false_clause_var));
    searcher.add(ENodeOrVar::ENode(Expression::Conditional {
        children: [condition_id, true_clause_id, false_clause_id],
    }));

    let mut applier = egg::RecExpr::default();
    let condition_id = applier.add(searcher[condition_id].clone());
    let true_branch_id = applier.add(searcher[block_id].clone());
    let false_value_id = applier.add(searcher[false_clause_id].clone());
    let false_branch_id = applier.add(ENodeOrVar::ENode(Expression::BlockReturn {
        children: [false_value_id],
    }));
    let if_block_id = applier.add(ENodeOrVar::ENode(Expression::BlockIf {
        children: [condition_id, true_branch_id, false_branch_id],
    }));
    let arrow_function_id = applier.add(ENodeOrVar::ENode(Expression::ArrowFunctionBlock0 {
        children: [if_block_id],
    }));
    applier.add(ENodeOrVar::ENode(Expression::Call {
        children: vec![arrow_function_id], // no arguments
    }));

    Rewrite::new(
        egg::Symbol::from("ternary_with_iife_true_clause_to_block"),
        egg::Pattern::new(searcher),
        egg::Pattern::new(applier),
    )
    .unwrap()
}

// A ternary with an IIFE false clause can be rewritten as an IIFE containing an `if` block.
//
//      condition ? 5 : (() => { block })()
//      👉 (() => { if (condition) { return 5 } block })()
//
fn rewrite_ternary_with_iife_false_clause_to_block() -> Rewrite {
    let condition_var = Var::from_str("?condition").unwrap();
    let block_var = Var::from_str("?block").unwrap();
    let true_clause_var = Var::from_str("?true_clause").unwrap();

    let mut searcher = egg::RecExpr::default();
    let condition_id = searcher.add(ENodeOrVar::Var(condition_var));
    let true_clause_id = searcher.add(ENodeOrVar::Var(true_clause_var));
    let block_id = searcher.add(ENodeOrVar::Var(block_var));
    let arrow_function_id = searcher.add(ENodeOrVar::ENode(Expression::ArrowFunctionBlock0 {
        children: [block_id],
    }));
    let false_clause_id = searcher.add(ENodeOrVar::ENode(Expression::Call {
        children: vec![arrow_function_id], // no arguments
    }));
    searcher.add(ENodeOrVar::ENode(Expression::Conditional {
        children: [condition_id, true_clause_id, false_clause_id],
    }));

    let mut applier = egg::RecExpr::default();
    let condition_id = applier.add(searcher[condition_id].clone());
    let true_value_id = applier.add(searcher[true_clause_id].clone());
    let true_branch_id = applier.add(ENodeOrVar::ENode(Expression::BlockReturn {
        children: [true_value_id],
    }));

    let false_branch_id = applier.add(searcher[block_id].clone());
    let if_block_id = applier.add(ENodeOrVar::ENode(Expression::BlockIf {
        children: [condition_id, true_branch_id, false_branch_id],
    }));
    let arrow_function_id = applier.add(ENodeOrVar::ENode(Expression::ArrowFunctionBlock0 {
        children: [if_block_id],
    }));
    applier.add(ENodeOrVar::ENode(Expression::Call {
        children: vec![arrow_function_id], // no arguments
    }));

    Rewrite::new(
        egg::Symbol::from("ternary_with_iife_false_clause_to_block"),
        egg::Pattern::new(searcher),
        egg::Pattern::new(applier),
    )
    .unwrap()
}

// Returning an IIFE is pointless, just inline the block.
//
//      return (() => { block })()
//      👉 block
//
fn rewrite_inline_returned_iife_block() -> Rewrite {
    let block_var = Var::from_str("?block").unwrap();

    let mut searcher = egg::RecExpr::default();
    let block_id = searcher.add(ENodeOrVar::Var(block_var));
    let arrow_function_id = searcher.add(ENodeOrVar::ENode(Expression::ArrowFunctionBlock0 {
        children: [block_id],
    }));
    let iife_id = searcher.add(ENodeOrVar::ENode(Expression::Call {
        children: vec![arrow_function_id], // no arguments
    }));
    searcher.add(ENodeOrVar::ENode(Expression::BlockReturn {
        children: [iife_id],
    }));

    let mut applier = egg::RecExpr::default();
    applier.add(searcher[block_id].clone());

    Rewrite::new(
        egg::Symbol::from("inline_returned_iife_block"),
        egg::Pattern::new(searcher),
        egg::Pattern::new(applier),
    )
    .unwrap()
}

// `return undefined` in JavaScript is redundant
fn rewrite_redundant_return_undefined() -> Rewrite {
    let mut searcher = egg::RecExpr::default();
    let undefined_id = searcher.add(ENodeOrVar::ENode(Expression::Undefined));
    searcher.add(ENodeOrVar::ENode(Expression::BlockReturn {
        children: [undefined_id],
    }));

    let mut applier = egg::RecExpr::default();
    applier.add(ENodeOrVar::ENode(Expression::BlockReturnVoid));

    Rewrite::new(
        egg::Symbol::from("redundant_return_undefined"),
        egg::Pattern::new(searcher),
        egg::Pattern::new(applier),
    )
    .unwrap()
}

// Immediately invoked identity functions can be replaced with the argument.
//
//      ((x) => x)(y)
//      👉 y
//
fn rewrite_inline_immediate_identity_call() -> Rewrite {
    let expr_var = Var::from_str("?var").unwrap();

    let mut searcher = egg::RecExpr::default();
    let identity_id = searcher.add(ENodeOrVar::ENode(Expression::ArrowFunctionIdentity(Ident(
        // NOTE: this identifier is ignored during Expression matching so can be anything
        "whatever".to_string(),
    ))));
    let expr_id = searcher.add(ENodeOrVar::Var(expr_var));
    searcher.add(ENodeOrVar::ENode(Expression::Call {
        children: vec![identity_id, expr_id],
    }));

    let mut applier = egg::RecExpr::default();
    applier.add(searcher[expr_id].clone());

    Rewrite::new(
        egg::Symbol::from("inline_immediate_identity_call"),
        egg::Pattern::new(searcher),
        egg::Pattern::new(applier),
    )
    .unwrap()
}

// Prefer calling the arms of a tenary rather than the whole thing wrapped in parens.
//
//      (cond ? f : g)()
//      👉 cond ? f() : g()
//
fn rewrite_ternary_call0() -> Rewrite {
    let condition_var = Var::from_str("?cond").unwrap();
    let true_clause_var = Var::from_str("?true_clause").unwrap();
    let false_clause_var = Var::from_str("?false_clause").unwrap();

    let mut searcher = egg::RecExpr::default();

    let condition_id = searcher.add(ENodeOrVar::Var(condition_var));
    let true_clause_id = searcher.add(ENodeOrVar::Var(true_clause_var));
    let false_clause_id = searcher.add(ENodeOrVar::Var(false_clause_var));
    let ternary_id = searcher.add(ENodeOrVar::ENode(Expression::Conditional {
        children: [condition_id, true_clause_id, false_clause_id],
    }));
    searcher.add(ENodeOrVar::ENode(Expression::Call {
        children: vec![ternary_id],
    }));

    let mut applier = egg::RecExpr::default();
    let condition_id = applier.add(searcher[condition_id].clone());
    let true_clause_id = applier.add(searcher[true_clause_id].clone());
    let true_clause_id = applier.add(ENodeOrVar::ENode(Expression::Call {
        children: vec![true_clause_id],
    }));
    let false_clause_id = applier.add(searcher[false_clause_id].clone());
    let false_clause_id = applier.add(ENodeOrVar::ENode(Expression::Call {
        children: vec![false_clause_id],
    }));
    applier.add(ENodeOrVar::ENode(Expression::Conditional {
        children: [condition_id, true_clause_id, false_clause_id],
    }));

    Rewrite::new(
        egg::Symbol::from("ternary_call0"),
        egg::Pattern::new(searcher),
        egg::Pattern::new(applier),
    )
    .unwrap()
}

// Rewrite a redundant (nullary) arrow block body.
//
//      () => { return x }
//      👉 () => x
//
fn rewrite_redundant_arrow0_block() -> Rewrite {
    let expr_var = Var::from_str("?expr").unwrap();

    let mut searcher = egg::RecExpr::default();
    let expr_id = searcher.add(ENodeOrVar::Var(expr_var));
    let return_id = searcher.add(ENodeOrVar::ENode(Expression::BlockReturn {
        children: [expr_id],
    }));
    searcher.add(ENodeOrVar::ENode(Expression::ArrowFunctionBlock0 {
        children: [return_id],
    }));

    let mut applier = egg::RecExpr::default();
    let expr_id = applier.add(searcher[expr_id].clone());
    applier.add(ENodeOrVar::ENode(Expression::ArrowFunctionExpr0 {
        children: [expr_id],
    }));

    Rewrite::new(
        egg::Symbol::from("redundant_arrow0_block"),
        egg::Pattern::new(searcher),
        egg::Pattern::new(applier),
    )
    .unwrap()
}

// Rewrite a redundant arrow block body.
//
//      (...args) => { return x }
//      👉 (...args) => x
//
fn rewrite_redundant_arrow_block() -> Rewrite {
    let expr_var = Var::from_str("?expr").unwrap();
    let params_var = Var::from_str("?params").unwrap();

    let mut searcher = egg::RecExpr::default();
    let params_id = searcher.add(ENodeOrVar::Var(params_var));
    let expr_id = searcher.add(ENodeOrVar::Var(expr_var));
    let return_id = searcher.add(ENodeOrVar::ENode(Expression::BlockReturn {
        children: [expr_id],
    }));
    searcher.add(ENodeOrVar::ENode(Expression::ArrowFunctionBlock {
        children: [params_id, return_id],
    }));

    let mut applier = egg::RecExpr::default();
    let params_id = applier.add(searcher[params_id].clone());
    let expr_id = applier.add(searcher[expr_id].clone());
    applier.add(ENodeOrVar::ENode(Expression::ArrowFunctionExpr {
        children: [params_id, expr_id],
    }));

    Rewrite::new(
        egg::Symbol::from("redundant_arrow_block"),
        egg::Pattern::new(searcher),
        egg::Pattern::new(applier),
    )
    .unwrap()
}

// Rewrite a redundant IIFE
//
//      (() => x)()
//      👉 x
//
fn rewrite_redundant_iife() -> Rewrite {
    let expr_var = Var::from_str("?expr").unwrap();

    let mut searcher = egg::RecExpr::default();
    let expr_id = searcher.add(ENodeOrVar::Var(expr_var));
    let arrow_id = searcher.add(ENodeOrVar::ENode(Expression::ArrowFunctionExpr0 {
        children: [expr_id],
    }));
    searcher.add(ENodeOrVar::ENode(Expression::Call {
        children: vec![arrow_id],
    }));

    let mut applier = egg::RecExpr::default();
    applier.add(searcher[expr_id].clone());

    Rewrite::new(
        egg::Symbol::from("redundant_iife"),
        egg::Pattern::new(searcher),
        egg::Pattern::new(applier),
    )
    .unwrap()
}

#[cfg(test)]
mod tests {
    use crate::optimize::test_macros::assert_optimized;

    #[test]
    fn it_rewrites_ifs() {
        assert_optimized!("if true then 1 else 2", "1");
        assert_optimized!(r#"if false then "yeh" else "nah""#, r#""nah""#);
        assert_optimized!("if true then (if false then 1 else 2) else 3", "2");
        assert_optimized!("[if true then 1 else 2, if false then 3 else 2]", "[1,2]");
        assert_optimized!("if true then true else false", "true");
        assert_optimized!("(x: Bool) -> if x then true else false", "(x) => x");
        assert_optimized!(
            "(cond, fn) -> (if cond then fn else fn)()",
            "(cond,fn) => cond?fn():fn()"
        );
    }

    #[test]
    fn it_rewrites_matches() {
        assert_optimized!(
            "(x) -> match x with | y -> y",
            "(x) => {const y = x;return y;}"
        );
        assert_optimized!(
            "(bc) -> match bc with | B -> 1 | C -> 2",
            "(bc) => {if ((bc[0] === \"B\")){return 1;}if ((bc[0] === \"C\")){return 2;}throw new Error(\"Pattern match error\");}"
        );
    }

    #[test]
    fn it_rewrites_effects() {
        assert_optimized!("do { return unit }", "() => undefined");
        assert_optimized!("do { return 5 }", "() => 5");
    }

    #[test]
    fn it_removes_redundant_return_unit() {
        assert_optimized!("(x) -> do { return unit }", "(x) => () => {return;}");
    }

    #[test]
    fn it_removes_redundant_iifes() {
        assert_optimized!("(() -> 5)()", "5");
    }

    #[test]
    fn it_inlines_identity_calls() {
        assert_optimized!("((x) -> x)(5)", "5");
        assert_optimized!("((x) -> x)((a) -> a)", "(a) => a");
    }
}
