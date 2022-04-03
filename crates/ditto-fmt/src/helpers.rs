use dprint_core::formatting::{
    condition_resolvers, conditions, ir_helpers, Condition, PrintItems, Signal,
};

pub fn space() -> PrintItems {
    // REVIEW it would be nice if this handled the case where a previous trailing comment forces
    // a newline (in which case this should be a single indent)
    " ".into()
}

pub fn group(items: PrintItems, force_use_new_lines: bool) -> PrintItems {
    let mut group_items = PrintItems::new();
    group_items.push_signal(if force_use_new_lines {
        Signal::NewLine
    } else {
        Signal::SpaceOrNewLine
    });
    group_items.push_condition(indent_if_start_of_line_or_start_of_line_indented(items));
    group_items
}

fn indent_if_start_of_line_or_start_of_line_indented(items: PrintItems) -> Condition {
    let rc_path = items.into_rc_path();
    conditions::if_true_or(
        "withIndentIfStartOfLineOrStartOfLineIndented",
        |context| {
            Some(
                condition_resolvers::is_start_of_line_indented(context)
                    || condition_resolvers::is_start_of_line(context),
            )
        },
        ir_helpers::with_indent(rc_path.into()),
        rc_path.into(),
    )
}

fn _resolver_or<R>(lhs: Option<bool>, rhs: R) -> Option<bool>
where
    R: FnOnce() -> Option<bool>,
{
    match lhs {
        Some(true) => Some(true),
        _ => rhs(),
    }
}
