//! Garbage / parse-failure detection.
//!
//! These helpers walk the AST searching for `Expr::Garbage` nodes,
//! which indicate regions the parser could not understand. The
//! formatter uses this information to decide whether a block can be
//! safely reformatted or must be emitted verbatim.

use nu_protocol::ast::{
    Argument, Block, Expr, Expression, ExternalArgument, ListItem, MatchPattern, Pattern, Pipeline,
    PipelineRedirection, RecordItem, RedirectionTarget,
};
use nu_protocol::engine::StateWorkingSet;

/// Check if a block contains any garbage expressions.
pub(super) fn block_contains_garbage(working_set: &StateWorkingSet<'_>, block: &Block) -> bool {
    block
        .pipelines
        .iter()
        .any(|pipeline| pipeline_contains_garbage(working_set, pipeline))
}

/// Check if a pipeline contains garbage expressions.
fn pipeline_contains_garbage(working_set: &StateWorkingSet<'_>, pipeline: &Pipeline) -> bool {
    pipeline.elements.iter().any(|element| {
        expr_contains_garbage(working_set, &element.expr)
            || element
                .redirection
                .as_ref()
                .is_some_and(|redir| redirection_contains_garbage(working_set, redir))
    })
}

/// Check if a redirection contains garbage expressions.
fn redirection_contains_garbage(
    working_set: &StateWorkingSet<'_>,
    redir: &PipelineRedirection,
) -> bool {
    match redir {
        PipelineRedirection::Single { target, .. } => {
            redirection_target_contains_garbage(working_set, target)
        }
        PipelineRedirection::Separate { out, err } => {
            redirection_target_contains_garbage(working_set, out)
                || redirection_target_contains_garbage(working_set, err)
        }
    }
}

/// Check if a redirection target contains garbage.
fn redirection_target_contains_garbage(
    working_set: &StateWorkingSet<'_>,
    target: &RedirectionTarget,
) -> bool {
    match target {
        RedirectionTarget::File { expr, .. } => expr_contains_garbage(working_set, expr),
        RedirectionTarget::Pipe { .. } => false,
    }
}

/// Check if a call argument contains garbage expressions.
fn argument_contains_garbage(working_set: &StateWorkingSet<'_>, arg: &Argument) -> bool {
    match arg {
        Argument::Positional(expr) | Argument::Unknown(expr) | Argument::Spread(expr) => {
            expr_contains_garbage(working_set, expr)
        }
        Argument::Named(named) => named
            .2
            .as_ref()
            .is_some_and(|expr| expr_contains_garbage(working_set, expr)),
    }
}

/// Check if any expression in the tree contains garbage nodes.
pub(super) fn expr_contains_garbage(working_set: &StateWorkingSet<'_>, expr: &Expression) -> bool {
    match &expr.expr {
        Expr::Garbage => true,
        Expr::Call(call) => call
            .arguments
            .iter()
            .any(|arg| argument_contains_garbage(working_set, arg)),
        Expr::ExternalCall(head, args) => {
            expr_contains_garbage(working_set, head)
                || args.iter().any(|arg| match arg {
                    ExternalArgument::Regular(expr) | ExternalArgument::Spread(expr) => {
                        expr_contains_garbage(working_set, expr)
                    }
                })
        }
        Expr::BinaryOp(lhs, op, rhs) => {
            expr_contains_garbage(working_set, lhs)
                || expr_contains_garbage(working_set, op)
                || expr_contains_garbage(working_set, rhs)
        }
        Expr::UnaryNot(inner) => expr_contains_garbage(working_set, inner),
        Expr::Block(block_id) | Expr::Closure(block_id) | Expr::Subexpression(block_id) => {
            block_contains_garbage(working_set, working_set.get_block(*block_id))
        }
        Expr::Range(range) => {
            range
                .from
                .as_ref()
                .is_some_and(|e| expr_contains_garbage(working_set, e))
                || range
                    .next
                    .as_ref()
                    .is_some_and(|e| expr_contains_garbage(working_set, e))
                || range
                    .to
                    .as_ref()
                    .is_some_and(|e| expr_contains_garbage(working_set, e))
        }
        Expr::List(items) => items.iter().any(|item| match item {
            ListItem::Item(expr) | ListItem::Spread(_, expr) => {
                expr_contains_garbage(working_set, expr)
            }
        }),
        Expr::Record(items) => items.iter().any(|item| match item {
            RecordItem::Pair(k, v) => {
                expr_contains_garbage(working_set, k) || expr_contains_garbage(working_set, v)
            }
            RecordItem::Spread(_, expr) => expr_contains_garbage(working_set, expr),
        }),
        Expr::Table(table) => {
            table
                .columns
                .iter()
                .any(|col| expr_contains_garbage(working_set, col))
                || table
                    .rows
                    .iter()
                    .flat_map(|row| row.iter())
                    .any(|cell| expr_contains_garbage(working_set, cell))
        }
        Expr::CellPath(_) => false,
        Expr::FullCellPath(full_path) => expr_contains_garbage(working_set, &full_path.head),
        Expr::RowCondition(block_id) => {
            block_contains_garbage(working_set, working_set.get_block(*block_id))
        }
        Expr::Keyword(keyword) => expr_contains_garbage(working_set, &keyword.expr),
        Expr::MatchBlock(matches) => matches.iter().any(|(pattern, arm_expr)| {
            pattern_contains_garbage(working_set, pattern)
                || expr_contains_garbage(working_set, arm_expr)
        }),
        Expr::Collect(_, inner) => expr_contains_garbage(working_set, inner),
        Expr::AttributeBlock(attr_block) => {
            attr_block
                .attributes
                .iter()
                .any(|attr| expr_contains_garbage(working_set, &attr.expr))
                || expr_contains_garbage(working_set, &attr_block.item)
        }
        Expr::Int(_)
        | Expr::Float(_)
        | Expr::Bool(_)
        | Expr::Nothing
        | Expr::DateTime(_)
        | Expr::String(_)
        | Expr::RawString(_)
        | Expr::Binary(_)
        | Expr::Filepath(_, _)
        | Expr::Directory(_, _)
        | Expr::GlobPattern(_, _)
        | Expr::Var(_)
        | Expr::VarDecl(_)
        | Expr::Operator(_)
        | Expr::StringInterpolation(_)
        | Expr::GlobInterpolation(_, _)
        | Expr::ImportPattern(_)
        | Expr::Overlay(_)
        | Expr::Signature(_)
        | Expr::ValueWithUnit(_) => false,
    }
}

/// Check if a match pattern contains garbage.
fn pattern_contains_garbage(working_set: &StateWorkingSet<'_>, pattern: &MatchPattern) -> bool {
    match &pattern.pattern {
        Pattern::Garbage => true,
        Pattern::Expression(expr) => expr_contains_garbage(working_set, expr),
        Pattern::Or(patterns) | Pattern::List(patterns) => patterns
            .iter()
            .any(|pattern| pattern_contains_garbage(working_set, pattern)),
        Pattern::Record(entries) => entries
            .iter()
            .any(|(_, pattern)| pattern_contains_garbage(working_set, pattern)),
        Pattern::Value(_)
        | Pattern::Variable(_)
        | Pattern::Rest(_)
        | Pattern::IgnoreRest
        | Pattern::IgnoreValue => false,
    }
}
